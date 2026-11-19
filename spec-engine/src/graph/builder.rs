use crate::graph::model::{
    BrokenTrace, CoverageViolation, GraphReport, NodeKind, RequirementEntry, TraceEdge, TraceNode,
    TraceabilityReport,
};
use crate::policy::{EdgeKind, Policy, Severity};
use crate::scanner::lexer::{Priority, TagKind};
use crate::scanner::ScannedTag;
use std::collections::{HashMap, HashSet};

// @impl:FR-CLI-GRAPH-01
/// Builds the bidirectional traceability graph and checks closure for MUST requirements.
///
/// # Invariants: Every requirement marked MUST must possess design, impl, and test to close the graph.
pub fn build_trace_graph(tags: &[ScannedTag]) -> TraceabilityReport {
    let mut requirements: HashMap<String, RequirementEntry> = HashMap::new();
    let mut designs: HashMap<String, Vec<&ScannedTag>> = HashMap::new();
    let mut impls: HashMap<String, Vec<&ScannedTag>> = HashMap::new();
    let mut tests: HashMap<String, Vec<&ScannedTag>> = HashMap::new();

    // 1. Group occurrences by category
    for tag in tags {
        match &tag.kind {
            TagKind::Req { id, priority } => {
                requirements
                    .entry(id.clone())
                    .or_insert_with(|| RequirementEntry {
                        id: id.clone(),
                        priority: *priority,
                        file_path: tag.file_path.clone(),
                        line_number: tag.line_number,
                        has_design: false,
                        has_impl: false,
                        has_test: false,
                    });
            }
            TagKind::Design { id } => {
                designs.entry(id.clone()).or_default().push(tag);
            }
            TagKind::Impl { id } => {
                impls.entry(id.clone()).or_default().push(tag);
            }
            TagKind::Test { id } => {
                tests.entry(id.clone()).or_default().push(tag);
            }
            _ => {}
        }
    }

    let mut must_satisfied = 0;
    let mut must_broken = Vec::new();
    let mut should_pending = Vec::new();
    let mut draft_total = 0;
    let mut draft_pending = Vec::new();

    // 2. Resolve closure for each requirement
    for req in requirements.values_mut() {
        req.has_design = designs.contains_key(&req.id);
        req.has_impl = impls.contains_key(&req.id);
        req.has_test = tests.contains_key(&req.id);

        let is_complete = req.has_impl && req.has_test;

        match req.priority {
            Priority::Must => {
                if is_complete {
                    must_satisfied += 1;
                } else {
                    must_broken.push(BrokenTrace {
                        id: req.id.clone(),
                        file_path: req.file_path.clone(),
                        line_number: req.line_number,
                        missing_design: !req.has_design,
                        missing_impl: !req.has_impl,
                        missing_test: !req.has_test,
                    });
                }
            }
            Priority::Should => {
                if !is_complete {
                    should_pending.push(BrokenTrace {
                        id: req.id.clone(),
                        file_path: req.file_path.clone(),
                        line_number: req.line_number,
                        missing_design: !req.has_design,
                        missing_impl: !req.has_impl,
                        missing_test: !req.has_test,
                    });
                }
            }
            Priority::Draft => {
                draft_total += 1;
                if !is_complete {
                    draft_pending.push(BrokenTrace {
                        id: req.id.clone(),
                        file_path: req.file_path.clone(),
                        line_number: req.line_number,
                        missing_design: !req.has_design,
                        missing_impl: !req.has_impl,
                        missing_test: !req.has_test,
                    });
                }
            }
        }
    }

    // Sort broken lists by ID for deterministic output
    must_broken.sort_by(|a, b| a.id.cmp(&b.id));
    should_pending.sort_by(|a, b| a.id.cmp(&b.id));
    draft_pending.sort_by(|a, b| a.id.cmp(&b.id));

    let is_closed = must_broken.is_empty();

    TraceabilityReport {
        total_requirements: requirements.len(),
        must_satisfied,
        must_broken,
        should_pending,
        draft_total,
        draft_pending,
        is_closed,
    }
}

// @impl:FR-CLI-GRAPH-02
/// Builds the typed relation graph from scanned tags and evaluates the
/// canonical coverage rules COV-01..COV-06 from `policy`.
///
/// Nodes are collected first (unique per `(NodeKind, id)`, first occurrence
/// wins), then edges from `TagKind::Rel`. Undeclared edge endpoints and
/// duplicate IDs within a `NodeKind` are reported as COV-06. Coverage rules
/// are evaluated from `policy.coverage_rules()`; `verifies` edges require a
/// non-empty note (COV-05). Violations are sorted by `(rule_id, subject)`.
///
/// `is_closed` is true when no error-severity violation exists.
pub fn build_relation_graph(tags: &[ScannedTag], policy: &Policy) -> GraphReport {
    // 1. Collect nodes: unique per (kind, id), first occurrence wins.
    let mut nodes: Vec<TraceNode> = Vec::new();
    let mut decl_counts: HashMap<(NodeKind, String), usize> = HashMap::new();
    let mut declared_ids: HashMap<String, NodeKind> = HashMap::new();

    for tag in tags {
        let (kind, id) = match &tag.kind {
            TagKind::Req { id, .. } => (NodeKind::Req, id.clone()),
            TagKind::Design { id } => (NodeKind::Design, id.clone()),
            TagKind::Impl { id } => (NodeKind::Impl, id.clone()),
            TagKind::Test { id } => (NodeKind::Test, id.clone()),
            TagKind::Adr { id } => (NodeKind::Adr, id.clone()),
            _ => continue,
        };
        *decl_counts.entry((kind, id.clone())).or_default() += 1;
        if declared_ids.contains_key(&id) {
            continue;
        }
        let node = TraceNode {
            kind,
            id: id.clone(),
            file_path: tag.file_path.clone(),
            line_number: tag.line_number,
        };
        declared_ids.insert(id.clone(), kind);
        nodes.push(node);
    }
    nodes.sort_by(|a, b| a.kind.cmp(&b.kind).then_with(|| a.id.cmp(&b.id)));

    // 2. Collect edges from Rel tags, in source order.
    //    A multi-target relation is flattened into one edge per target so the
    //    graph is a plain many-to-many relation (one target per edge).
    let mut edges: Vec<TraceEdge> = Vec::new();
    for tag in tags {
        if let TagKind::Rel { kind, from, to, note } = &tag.kind {
            for target in to {
                edges.push(TraceEdge {
                    kind: *kind,
                    from: from.clone(),
                    to: vec![target.clone()],
                    note: note.clone(),
                });
            }
        }
    }

    // 3. COV-06: dangling endpoints and duplicate IDs.
    //    Violation when found > required (zero tolerance for both).
    let mut violations: Vec<CoverageViolation> = Vec::new();

    for edge in &edges {
        if !declared_ids.contains_key(&edge.from) {
            violations.push(CoverageViolation {
                rule_id: "COV-06".to_string(),
                subject: edge.from.clone(),
                found: 1,
                required: 0,
            });
        }
        for target in &edge.to {
            if !declared_ids.contains_key(target) {
                violations.push(CoverageViolation {
                    rule_id: "COV-06".to_string(),
                    subject: target.clone(),
                    found: 1,
                    required: 0,
                });
            }
        }
    }
    for ((_kind, id), count) in &decl_counts {
        if *count > 1 {
            violations.push(CoverageViolation {
                rule_id: "COV-06".to_string(),
                subject: id.clone(),
                found: *count,
                required: 1,
            });
        }
    }

    // Edge indexes for coverage rules COV-01, COV-02, COV-04, COV-05.
    let mut incoming_satisfies: HashMap<String, usize> = HashMap::new();
    let mut incoming_verifies: HashMap<String, usize> = HashMap::new();
    let mut satisfies_to_req: HashSet<String> = HashSet::new(); // source -> at least one Req target
    let mut informs_to_adr: HashSet<String> = HashSet::new();   // source -> at least one Adr target
    let mut verifies_with_note: HashMap<String, usize> = HashMap::new();

    for edge in &edges {
        for target in &edge.to {
            match edge.kind {
                EdgeKind::Satisfies => {
                    *incoming_satisfies.entry(target.clone()).or_default() += 1;
                }
                EdgeKind::Verifies => {
                    *incoming_verifies.entry(target.clone()).or_default() += 1;
                }
                EdgeKind::Informs | EdgeKind::Constrains => {}
            }
        }
        match edge.kind {
            EdgeKind::Satisfies => {
                if edge.to.iter().any(|t| declared_ids.get(t) == Some(&NodeKind::Req)) {
                    satisfies_to_req.insert(edge.from.clone());
                }
            }
            EdgeKind::Informs => {
                if edge.to.iter().any(|t| declared_ids.get(t) == Some(&NodeKind::Adr)) {
                    informs_to_adr.insert(edge.from.clone());
                }
            }
            EdgeKind::Verifies => {
                if edge.note.is_some() {
                    *verifies_with_note.entry(edge.from.clone()).or_default() += 1;
                }
            }
            EdgeKind::Constrains => {}
        }
    }

    // 4. COV-01..COV-05 from policy.coverage_rules().
    //    Violation when found < required (per rule min).
    let rule_min = |rule_id: &str| -> usize {
        policy
            .coverage_rules()
            .iter()
            .find(|r| r.id == rule_id)
            .map(|r| r.min)
            .unwrap_or(0)
    };

    for node in &nodes {
        match node.kind {
            NodeKind::Req => {
                let priority = match tags.iter().find_map(|t| match &t.kind {
                    TagKind::Req { id, priority } if id == &node.id => Some(*priority),
                    _ => None,
                }) {
                    Some(p) => p,
                    None => continue,
                };
                match priority {
                    Priority::Draft => continue,
                    Priority::Must => {
                        let sat = incoming_satisfies.get(&node.id).copied().unwrap_or(0) > 0;
                        let ver = incoming_verifies.get(&node.id).copied().unwrap_or(0) > 0;
                        if !sat {
                            violations.push(CoverageViolation {
                                rule_id: "COV-01".to_string(),
                                subject: node.id.clone(),
                                found: incoming_satisfies.get(&node.id).copied().unwrap_or(0),
                                required: rule_min("COV-01"),
                            });
                        }
                        if !ver {
                            violations.push(CoverageViolation {
                                rule_id: "COV-01".to_string(),
                                subject: node.id.clone(),
                                found: incoming_verifies.get(&node.id).copied().unwrap_or(0),
                                required: rule_min("COV-01"),
                            });
                        }
                    }
                    Priority::Should => {
                        let sat = incoming_satisfies.get(&node.id).copied().unwrap_or(0) > 0;
                        let ver = incoming_verifies.get(&node.id).copied().unwrap_or(0) > 0;
                        if !sat && !ver {
                            violations.push(CoverageViolation {
                                rule_id: "COV-02".to_string(),
                                subject: node.id.clone(),
                                found: 0,
                                required: rule_min("COV-02"),
                            });
                        }
                    }
                }
            }
            NodeKind::Impl => {
                if !satisfies_to_req.contains(&node.id) && !informs_to_adr.contains(&node.id) {
                    violations.push(CoverageViolation {
                        rule_id: "COV-04".to_string(),
                        subject: node.id.clone(),
                        found: 0,
                        required: rule_min("COV-04"),
                    });
                }
            }
            NodeKind::Test => {
                let found = verifies_with_note.get(&node.id).copied().unwrap_or(0);
                if found < rule_min("COV-05") {
                    violations.push(CoverageViolation {
                        rule_id: "COV-05".to_string(),
                        subject: node.id.clone(),
                        found,
                        required: rule_min("COV-05"),
                    });
                }
            }
            NodeKind::Design | NodeKind::Adr => {}
        }
    }

    violations.sort_by(|a, b| a.rule_id.cmp(&b.rule_id).then_with(|| a.subject.cmp(&b.subject)));

    let error_rules: HashSet<&str> = policy
        .coverage_rules()
        .iter()
        .filter(|r| r.severity == Severity::Error)
        .map(|r| r.id.as_str())
        .collect();
    let is_closed = !violations.iter().any(|v| error_rules.contains(v.rule_id.as_str()));

    GraphReport {
        nodes,
        edges,
        violations,
        is_closed,
    }
}
