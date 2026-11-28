use asep::adr::create_new_adr;
use std::fs;
use tempfile::tempdir;

// @test:FR-CLI-ADR-01
#[test]
fn test_create_new_adr_numbering_and_content() {
    let dir = tempdir().expect("tempdir created");
    let root = dir.path();

    // 1. First ADR in empty project should be 0001
    let adr1_path = create_new_adr(root, "Adopt PostgreSQL Database").expect("first adr succeeds");
    assert!(adr1_path.exists());
    assert_eq!(
        adr1_path.file_name().unwrap().to_str().unwrap(),
        "0001-adopt-postgresql-database.md"
    );

    let content1 = fs::read_to_string(&adr1_path).expect("read adr1");
    assert!(content1.contains("# ADR-0001: Adopt PostgreSQL Database"));
    assert!(content1.contains("**Status:** Proposed"));
    assert!(content1.contains("Context & Problem Statement"));
    assert!(content1.contains("Considered Alternatives"));
    assert!(content1.contains("Decision Outcome"));
    assert!(content1.contains("**Traceability:** @adr:0001"));

    // 2. Second ADR should be 0002
    let adr2_path = create_new_adr(root, "Use Redis For Cache").expect("second adr succeeds");
    assert!(adr2_path.exists());
    assert_eq!(
        adr2_path.file_name().unwrap().to_str().unwrap(),
        "0002-use-redis-for-cache.md"
    );

    let content2 = fs::read_to_string(&adr2_path).expect("read adr2");
    assert!(content2.contains("# ADR-0002: Use Redis For Cache"));
    assert!(content2.contains("**Traceability:** @adr:0002"));
}
