use asep::graph::{build_relation_graph, build_trace_graph, CoverageViolation, GraphReport, NodeKind, Priority, TagKind};
use asep::policy::{EdgeKind, load_policy, Policy};
use asep::scanner::ScannedTag;
use std::path::PathBuf;

// @test:FR-CLI-GRAPH-01
#[test]
fn test_traceability_graph_closure() {
    let dummy_path = PathBuf::from("dummy.rs");

    // 1. Closed MUST requirement (has req, design, impl, test)
    let closed_tags = vec![
        ScannedTag {
            file_path: dummy_path.clone(),
            kind: TagKind::Req {
                id: "FR-AUTH-01".to_string(),
                priority: Priority::Must,
            },
            line_number: 1,
            raw_line: "@req:FR-AUTH-01:MUST".to_string(),
        },
        ScannedTag {
            file_path: dummy_path.clone(),
            kind: TagKind::Design {
                id: "FR-AUTH-01".to_string(),
            },
            line_number: 10,
            raw_line: "@design:FR-AUTH-01".to_string(),
        },
        ScannedTag {
            file_path: dummy_path.clone(),
            kind: TagKind::Impl {
                id: "FR-AUTH-01".to_string(),
            },
            line_number: 20,
            raw_line: "// @impl:FR-AUTH-01".to_string(),
        },
        ScannedTag {
            file_path: dummy_path.clone(),
            kind: TagKind::Test {
                id: "FR-AUTH-01".to_string(),
            },
            line_number: 30,
            raw_line: "// @test:FR-AUTH-01".to_string(),
        },
    ];

    let report_closed = build_trace_graph(&closed_tags);
    assert_eq!(report_closed.total_requirements, 1);
    assert_eq!(report_closed.must_satisfied, 1);
    assert!(report_closed.must_broken.is_empty());
    assert!(report_closed.is_closed);

    // 2. Broken MUST requirement (missing impl & test)
    let broken_tags = vec![
        ScannedTag {
            file_path: dummy_path.clone(),
            kind: TagKind::Req {
                id: "FR-PAY-01".to_string(),
                priority: Priority::Must,
            },
            line_number: 5,
            raw_line: "@req:FR-PAY-01:MUST".to_string(),
        },
        ScannedTag {
            file_path: dummy_path.clone(),
            kind: TagKind::Design {
                id: "FR-PAY-01".to_string(),
            },
            line_number: 15,
            raw_line: "@design:FR-PAY-01".to_string(),
        },
    ];

    let report_broken = build_trace_graph(&broken_tags);
    assert_eq!(report_broken.total_requirements, 1);
    assert_eq!(report_broken.must_satisfied, 0);
    assert_eq!(report_broken.must_broken.len(), 1);
    assert_eq!(report_broken.must_broken[0].id, "FR-PAY-01");
    assert!(report_broken.must_broken[0].missing_impl);
    assert!(report_broken.must_broken[0].missing_test);
    assert!(!report_broken.is_closed);

    // 3. Incomplete DRAFT requirement (Velocity Mode / RAD)
    let draft_tags = vec![
        ScannedTag {
            file_path: dummy_path.clone(),
            kind: TagKind::Req {
                id: "FR-RAD-01".to_string(),
                priority: Priority::Draft,
            },
            line_number: 1,
            raw_line: "@req:FR-RAD-01:DRAFT".to_string(),
        },
        ScannedTag {
            file_path: dummy_path,
            kind: TagKind::Impl {
                id: "FR-RAD-01".to_string(),
            },
            line_number: 10,
            raw_line: "// @impl:FR-RAD-01".to_string(),
        },
    ];

    let report_draft = build_trace_graph(&draft_tags);
    assert_eq!(report_draft.total_requirements, 1);
    assert_eq!(report_draft.draft_total, 1);
    assert_eq!(report_draft.draft_pending.len(), 1);
    assert_eq!(report_draft.draft_pending[0].id, "FR-RAD-01");
    assert!(report_draft.draft_pending[0].missing_design);
    assert!(report_draft.draft_pending[0].missing_test);
    assert!(!report_draft.draft_pending[0].missing_impl);
    assert!(report_draft.must_broken.is_empty());
    assert!(report_draft.is_closed); // Drafts do not break closure
}

fn policy() -> Policy {
    load_policy()
}

fn scanned_tag(kind: TagKind, line: usize) -> ScannedTag {
    ScannedTag {
        file_path: PathBuf::from("src/dummy.rs"),
        kind,
        line_number: line,
        raw_line: String::new(),
    }
}

fn has_violation(report: &GraphReport, rule_id: &str) -> bool {
    report.violations.iter().any(|v| v.rule_id == rule_id)
}

// @test:FR-CLI-GRAPH-02
#[test]
fn test_relation_graph_many_to_many() {
    let policy = policy();
    let tags = vec![
        scanned_tag(TagKind::Req { id: "R-1".to_string(), priority: Priority::Must }, 1),
        scanned_tag(TagKind::Req { id: "R-2".to_string(), priority: Priority::Must }, 2),
        scanned_tag(TagKind::Impl { id: "I-1".to_string() }, 10),
        scanned_tag(TagKind::Test { id: "T-1".to_string() }, 20),
        scanned_tag(
            TagKind::Rel {
                kind: EdgeKind::Satisfies,
                from: "I-1".to_string(),
                to: vec!["R-1".to_string(), "R-2".to_string()],
                note: Some("I-1 implements R-1 and R-2".to_string()),
            },
            30,
        ),
        scanned_tag(
            TagKind::Rel {
                kind: EdgeKind::Verifies,
                from: "T-1".to_string(),
                to: vec!["R-1".to_string(), "R-2".to_string()],
                note: Some("T-1 covers R-1 and R-2".to_string()),
            },
            40,
        ),
    ];

    let report = build_relation_graph(&tags, &policy);
    assert_eq!(report.nodes.len(), 4);
    assert_eq!(report.edges.len(), 4);
    assert!(!has_violation(&report, "COV-01"));
    assert!(!has_violation(&report, "COV-05"));
    assert!(report.is_closed);
}

// @test:FR-CLI-GRAPH-03
#[test]
fn test_relation_graph_rejects_dangling_and_duplicate() {
    let policy = policy();
    let tags = vec![
        scanned_tag(TagKind::Req { id: "R-1".to_string(), priority: Priority::Must }, 1),
        scanned_tag(TagKind::Impl { id: "I-1".to_string() }, 10),
        scanned_tag(TagKind::Impl { id: "I-1".to_string() }, 11), // duplicate ID within NodeKind
        scanned_tag(
            TagKind::Rel {
                kind: EdgeKind::Satisfies,
                from: "I-1".to_string(),
                to: vec!["GHOST-99".to_string()],
                note: None,
            },
            20,
        ),
    ];

    let report = build_relation_graph(&tags, &policy);
    let cov06: Vec<&CoverageViolation> = report
        .violations
        .iter()
        .filter(|v| v.rule_id == "COV-06")
        .collect();
    assert_eq!(cov06.len(), 2, "expected two COV-06 violations: {report:?}");
    assert!(cov06.iter().any(|v| v.subject == "GHOST-99"), "dangling endpoint not flagged");
    assert!(cov06.iter().any(|v| v.subject == "I-1"), "duplicate ID not flagged");
}

// @test:FR-CLI-GRAPH-04
#[test]
fn test_cov05_requires_verifies_note() {
    let policy = policy();
    let tags = vec![
        scanned_tag(TagKind::Test { id: "T-1".to_string() }, 1),
        scanned_tag(TagKind::Req { id: "R-1".to_string(), priority: Priority::Must }, 2),
        scanned_tag(
            TagKind::Rel {
                kind: EdgeKind::Verifies,
                from: "T-1".to_string(),
                to: vec!["R-1".to_string()],
                note: None,
            },
            10,
        ),
    ];

    let report = build_relation_graph(&tags, &policy);
    let cov05: Vec<&CoverageViolation> = report
        .violations
        .iter()
        .filter(|v| v.rule_id == "COV-05")
        .collect();
    assert_eq!(cov05.len(), 1, "expected one COV-05 violation: {report:?}");
    assert_eq!(cov05[0].subject, "T-1");
    assert_eq!(cov05[0].found, 0);
    assert_eq!(cov05[0].required, 1);
}

// @test:FR-CLI-GRAPH-05
#[test]
fn test_relation_graph_allows_same_id_across_node_kinds() {
    let policy = policy();
    let tags = vec![
        scanned_tag(TagKind::Req { id: "X".to_string(), priority: Priority::Must }, 1),
        scanned_tag(TagKind::Impl { id: "X".to_string() }, 10),
        scanned_tag(TagKind::Test { id: "Xt".to_string() }, 20),
        scanned_tag(
            TagKind::Rel {
                kind: EdgeKind::Satisfies,
                from: "X".to_string(),
                to: vec!["X".to_string()],
                note: None,
            },
            30,
        ),
        scanned_tag(
            TagKind::Rel {
                kind: EdgeKind::Verifies,
                from: "Xt".to_string(),
                to: vec!["X".to_string()],
                note: Some("covers X".to_string()),
            },
            40,
        ),
    ];

    let report = build_relation_graph(&tags, &policy);
    // Req X and Impl X must be distinct nodes (IDs unique per NodeKind).
    let x_nodes: Vec<_> = report.nodes.iter().filter(|n| n.id == "X").collect();
    assert_eq!(x_nodes.len(), 2, "expected Req X and Impl X as distinct nodes: {report:?}");
    assert!(x_nodes.iter().any(|n| matches!(n.kind, NodeKind::Req)));
    assert!(x_nodes.iter().any(|n| matches!(n.kind, NodeKind::Impl)));
    // Cross-kind ID reuse must not produce COV-06.
    assert!(
        !report
            .violations
            .iter()
            .any(|v| v.rule_id == "COV-06"),
        "unexpected COV-06 for cross-kind ID reuse: {report:?}"
    );
    // COV-04 satisfied: Impl X has satisfies to Req X.
    assert!(
        !report
            .violations
            .iter()
            .any(|v| v.rule_id == "COV-04"),
        "unexpected COV-04: {report:?}"
    );
    // COV-01 satisfied: Req X has both satisfies and verifies incoming.
    assert!(
        !report
            .violations
            .iter()
            .any(|v| v.rule_id == "COV-01"),
        "unexpected COV-01: {report:?}"
    );
    // COV-05 satisfied: Test Xt has verifies with non-empty note.
    assert!(
        !report
            .violations
            .iter()
            .any(|v| v.rule_id == "COV-05"),
        "unexpected COV-05: {report:?}"
    );
    assert!(report.is_closed);
}
