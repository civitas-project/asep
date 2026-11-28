use asep::policy::EdgeKind;
use asep::scanner::lexer::{parse_tags_from_line, Priority, TagKind};

// @test:FR-CLI-SCAN-02
#[test]
fn test_parse_tags_from_various_languages() {
    // 1. Rust/Go/TS Requirement tag with MUST
    let line1 = "// @req:FR-AUTH-01:MUST";
    let tags1 = parse_tags_from_line(line1, 10);
    assert_eq!(tags1.len(), 1);
    assert_eq!(tags1[0].line_number, 10);
    assert_eq!(
        tags1[0].kind,
        TagKind::Req {
            id: "FR-AUTH-01".to_string(),
            priority: Priority::Must
        }
    );

    // 2. Requirement tag with SHOULD
    let line2 = "<!-- @req:FR-DOCS-02:SHOULD -->";
    let tags2 = parse_tags_from_line(line2, 25);
    assert_eq!(tags2.len(), 1);
    assert_eq!(
        tags2[0].kind,
        TagKind::Req {
            id: "FR-DOCS-02".to_string(),
            priority: Priority::Should
        }
    );

    // 2b. Requirement tag with DRAFT (Velocity Mode / RAD)
    let line2b = "<!-- @req:FR-RAD-01:DRAFT -->";
    let tags2b = parse_tags_from_line(line2b, 28);
    assert_eq!(tags2b.len(), 1);
    assert_eq!(
        tags2b[0].kind,
        TagKind::Req {
            id: "FR-RAD-01".to_string(),
            priority: Priority::Draft
        }
    );

    // 3. Technical Design tag
    let line3 = "/// @design:FR-AUTH-01";
    let tags3 = parse_tags_from_line(line3, 30);
    assert_eq!(tags3.len(), 1);
    assert_eq!(
        tags3[0].kind,
        TagKind::Design {
            id: "FR-AUTH-01".to_string()
        }
    );

    // 4. Implementation tag
    let line4 = "// @impl:FR-AUTH-01";
    let tags4 = parse_tags_from_line(line4, 40);
    assert_eq!(tags4.len(), 1);
    assert_eq!(
        tags4[0].kind,
        TagKind::Impl {
            id: "FR-AUTH-01".to_string()
        }
    );

    // 5. Automated test tag in Python style
    let line5 = "# @test:FR-AUTH-01";
    let tags5 = parse_tags_from_line(line5, 50);
    assert_eq!(tags5.len(), 1);
    assert_eq!(
        tags5[0].kind,
        TagKind::Test {
            id: "FR-AUTH-01".to_string()
        }
    );

    // 6. DUDUNG Token tag
    let line6 = "// @token:TK-COLOR-PRIMARY-500";
    let tags6 = parse_tags_from_line(line6, 60);
    assert_eq!(tags6.len(), 1);
    assert_eq!(
        tags6[0].kind,
        TagKind::Token {
            id: "TK-COLOR-PRIMARY-500".to_string()
        }
    );

    // 7. UJANG Governance tag
    let line7 = "<!-- @gov:ARCH-TOPOLOGY -->";
    let tags7 = parse_tags_from_line(line7, 70);
    assert_eq!(tags7.len(), 1);
    assert_eq!(
        tags7[0].kind,
        TagKind::Gov {
            category: "ARCH".to_string(),
            id: "TOPOLOGY".to_string()
        }
    );

    // 8. Plain code with no tags
    let line8 = "let x = 42;";
    let tags8 = parse_tags_from_line(line8, 80);
    assert!(tags8.is_empty());
}

// @test:FR-CLI-SCAN-02
#[test]
fn test_parse_adr_lim_criticality_tags() {
    let adr = parse_tags_from_line("// @adr:0001", 70);
    assert_eq!(adr.len(), 1);
    assert_eq!(
        adr[0].kind,
        TagKind::Adr {
            id: "0001".to_string()
        }
    );

    let lim = parse_tags_from_line("// @lim:LIM-MEM-01", 71);
    assert_eq!(lim.len(), 1);
    assert_eq!(
        lim[0].kind,
        TagKind::Limit {
            id: "LIM-MEM-01".to_string()
        }
    );

    let crit = parse_tags_from_line("// @criticality:high", 72);
    assert_eq!(crit.len(), 1);
    assert_eq!(
        crit[0].kind,
        TagKind::Criticality {
            level: "high".to_string()
        }
    );

    let prose = parse_tags_from_line("requires `@criticality:high|low` tagging", 73);
    assert_eq!(prose.len(), 1);
    assert_eq!(
        prose[0].kind,
        TagKind::Criticality {
            level: "high".to_string()
        }
    );
}

// @test:FR-CLI-SCAN-02
#[test]
fn test_parse_rel_tags_multi_target() {
    let tags = parse_tags_from_line("// @rel:satisfies:auth/session->AUTH-01,AUTH-02", 7);
    assert_eq!(tags.len(), 1);
    assert_eq!(
        tags[0].kind,
        TagKind::Rel {
            kind: EdgeKind::Satisfies,
            from: "auth/session".to_string(),
            to: vec!["AUTH-01".to_string(), "AUTH-02".to_string()],
            note: None,
        }
    );
}

// @test:FR-CLI-SCAN-02
#[test]
fn test_parse_rel_tags_with_note() {
    let tags = parse_tags_from_line(
        r#"// @rel:verifies:auth/session->AUTH-01  "token kedaluwarsa ditolak""#,
        7,
    );
    assert_eq!(tags.len(), 1);
    assert_eq!(
        tags[0].kind,
        TagKind::Rel {
            kind: EdgeKind::Verifies,
            from: "auth/session".to_string(),
            to: vec!["AUTH-01".to_string()],
            note: Some("token kedaluwarsa ditolak".to_string()),
        }
    );
}

// @test:FR-CLI-SCAN-02
#[test]
fn test_parse_rel_tags_unknown_kind_rejected() {
    let tags = parse_tags_from_line("// @rel:unknown:auth/session->AUTH-01", 7);
    assert!(tags.is_empty());
    assert!(tags.iter().all(|t| !matches!(t.kind, TagKind::Rel { .. })));
}
