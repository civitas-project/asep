use asep::scanner::{scan_project_tags, scan_project_tags_detailed, TagKind};
use std::fs;
use std::path::Path;
use tempfile::tempdir;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

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

// @test:FR-AUD-08
#[test]
fn scan_detailed_rejects_missing_root() {
    let result = scan_project_tags_detailed(Path::new("/nonexistent/asep-scan"));
    assert!(result.is_err());
}

#[test]
fn scan_detailed_reports_unreadable_file() {
    #[cfg(unix)]
    {
        let dir = tempdir().expect("tempdir created");
        let root = dir.path();
        let file = root.join("unreadable.rs");
        fs::write(&file, "// @impl:FR-AUD-08-TEST\n").expect("write file");
        fs::set_permissions(&file, PermissionsExt::from_mode(0o000)).expect("set permissions");

        let result = scan_project_tags_detailed(root).expect("detailed scan returns ok");
        assert!(
            !result.completeness.errors.is_empty(),
            "errors should be non-empty"
        );
        assert!(
            result
                .completeness
                .errors
                .iter()
                .any(|error| error.contains("unreadable.rs")),
            "errors should mention the filename"
        );

        fs::set_permissions(&file, PermissionsExt::from_mode(0o644)).expect("restore permissions");
    }
    #[cfg(not(unix))]
    {
        eprintln!("skip scan_detailed_reports_unreadable_file on non-unix");
    }
}

#[test]
fn scan_detailed_skips_binary_file() {
    let dir = tempdir().expect("tempdir created");
    let root = dir.path();
    let mut content = b"// @req:FR-BINARY-01\nprefix\n".to_vec();
    content.push(0);
    content.extend_from_slice(b"suffix\n");
    fs::write(root.join("binary.bin"), &content).expect("write binary file");

    let result = scan_project_tags_detailed(root).expect("detailed scan succeeds");
    assert!(result.completeness.skipped_binary >= 1);
    assert!(
        !result
            .tags
            .iter()
            .any(|t| matches!(&t.kind, TagKind::Req { id, .. } if id == "FR-BINARY-01")),
        "no tag from binary file"
    );
}

#[test]
fn scan_detailed_counts_files() {
    let dir = tempdir().expect("tempdir created");
    let root = dir.path();
    fs::write(root.join("a.rs"), "// @req:FR-AUD-08-A\n").expect("write a.rs");
    fs::write(root.join("b.rs"), "// @impl:FR-AUD-08-B\n").expect("write b.rs");

    let result = scan_project_tags_detailed(root).expect("detailed scan succeeds");
    assert!(result.completeness.files_seen >= 2);
    assert!(result.completeness.files_read >= 2);
    assert!(result.completeness.errors.is_empty());
}

#[test]
// @test:B08
fn incomplete_scan_is_an_error_for_all_consumers() {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempdir().unwrap();
        let root = dir.path();
        let file = root.join("unreadable.rs");
        fs::write(&file, "// @impl:FR-AUD-08-TEST
").unwrap();
        fs::set_permissions(&file, PermissionsExt::from_mode(0o000)).unwrap();
        let error = scan_project_tags(root).expect_err("incomplete scan must fail closed");
        assert!(error.to_string().contains("incomplete scan"));
        fs::set_permissions(&file, PermissionsExt::from_mode(0o644)).unwrap();
    }
}
