use asep::scanner::{scan_project_tags, TagKind};
use std::fs;
use tempfile::tempdir;

// @test:FR-CLI-SCAN-01
#[test]
fn test_scan_project_tags_respects_gitignore() {
    let dir = tempdir().expect("tempdir created");
    let root = dir.path();

    // 1. Create .gitignore
    fs::write(root.join(".gitignore"), "target/\n*.tmp\n").expect("write gitignore");

    // 2. Create tracked file in src/
    let src = root.join("src");
    fs::create_dir_all(&src).expect("create src dir");
    fs::write(src.join("lib.rs"), "// @impl:FR-ALPHA-01\npub fn work() {}").expect("write lib.rs");

    // 3. Create ignored directory target/
    let target = root.join("target");
    fs::create_dir_all(&target).expect("create target dir");
    fs::write(target.join("dummy.rs"), "// @impl:FR-IGNORED-01\n").expect("write dummy.rs");

    // 4. Create docs file
    let docs = root.join("docs");
    fs::create_dir_all(&docs).expect("create docs dir");
    fs::write(
        docs.join("PRD.md"),
        "# Requirements\n<!-- @req:FR-ALPHA-01:MUST -->\n",
    )
    .expect("write PRD.md");

    // Run scanner
    let tags = scan_project_tags(root).expect("scan succeeds");

    // Must find FR-ALPHA-01
    assert!(tags
        .iter()
        .any(|t| matches!(&t.kind, TagKind::Impl { id } if id == "FR-ALPHA-01")));
    assert!(tags
        .iter()
        .any(|t| matches!(&t.kind, TagKind::Req { id, .. } if id == "FR-ALPHA-01")));

    // Must NOT find FR-IGNORED-01
    assert!(!tags
        .iter()
        .any(|t| matches!(&t.kind, TagKind::Impl { id } if id == "FR-IGNORED-01")));
}
