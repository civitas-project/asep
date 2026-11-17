use crate::graph::model::{BrokenTrace, RequirementEntry, TraceabilityReport};
use crate::scanner::lexer::{Priority, TagKind};
use crate::scanner::ScannedTag;
use std::collections::HashMap;

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
