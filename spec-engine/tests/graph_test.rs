use asep::graph::{build_trace_graph, Priority, TagKind};
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
