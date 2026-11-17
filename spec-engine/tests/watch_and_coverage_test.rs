use asep::coverage::{map_coverage_to_requirements, parse_lcov, RequirementCoverageReport};
use asep::scanner::lexer::{Priority, TagKind};
use asep::scanner::ScannedTag;
use std::collections::HashMap;
use std::path::PathBuf;

#[test]
fn test_parse_lcov_format() {
    let lcov_content = r#"
TN:
SF:/mock/project/src/auth.rs
DA:10,2
DA:11,2
DA:12,0
DA:13,1
end_of_record
"#;

    let cov_map = parse_lcov(lcov_content);
    assert_eq!(cov_map.len(), 1);

    let auth_lines = cov_map.get(&PathBuf::from("/mock/project/src/auth.rs")).unwrap();
    assert_eq!(auth_lines.get(&10), Some(&2));
    assert_eq!(auth_lines.get(&11), Some(&2));
    assert_eq!(auth_lines.get(&12), Some(&0));
    assert_eq!(auth_lines.get(&13), Some(&1));
}

#[test]
fn test_map_coverage_to_requirements() {
    let mut cov_map = HashMap::new();
    let mut file_lines = HashMap::new();
    file_lines.insert(10, 5); // hit
    file_lines.insert(11, 0); // missed
    cov_map.insert(PathBuf::from("/mock/src/lib.rs"), file_lines);

    let tags = vec![
        ScannedTag {
            kind: TagKind::Req {
                id: "FR-AUTH-01".to_string(),
                priority: Priority::Must,
            },
            file_path: PathBuf::from("/mock/docs/PRD.md"),
            line_number: 5,
            raw_line: "@req:FR-AUTH-01:MUST".to_string(),
        },
        ScannedTag {
            kind: TagKind::Impl {
                id: "FR-AUTH-01".to_string(),
            },
            file_path: PathBuf::from("/mock/src/lib.rs"),
            line_number: 10,
            raw_line: "// @impl:FR-AUTH-01".to_string(),
        },
        ScannedTag {
            kind: TagKind::Impl {
                id: "FR-AUTH-01".to_string(),
            },
            file_path: PathBuf::from("/mock/src/lib.rs"),
            line_number: 11,
            raw_line: "// @impl:FR-AUTH-01".to_string(),
        },
    ];

    let report: RequirementCoverageReport = map_coverage_to_requirements(&tags, &cov_map, 80.0);
    assert_eq!(report.items.len(), 1);

    let item = &report.items[0];
    assert_eq!(item.req_id, "FR-AUTH-01");
    assert_eq!(item.total_lines, 2);
    assert_eq!(item.covered_lines, 1);
    assert_eq!(item.percentage, 50.0);
    assert!(!item.passed); // 50% < 80%
}
