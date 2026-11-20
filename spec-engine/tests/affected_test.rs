use asep::affected::{affected, AffectedNode, AffectedReport, AffectedTest};
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn write_tag_file(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&path, content).unwrap();
}

fn assert_empty(report: &AffectedReport) {
    assert!(report.impls.is_empty());
    assert!(report.requirements.is_empty());
    assert!(report.tests.is_empty());
    assert!(report.commands.is_empty());
}

#[test]
fn test_affected_graph_relations() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    write_tag_file(
        root,
        "docs/spec.md",
        "@req:FR-A:MUST\n@rel:satisfies:impl-a->FR-A\n@rel:verifies:test-a->FR-A \"note\"\n",
    );
    write_tag_file(root, "src/lib.rs", "// @impl:impl-a\n");
    write_tag_file(root, "tests/a_test.rs", "// @test:test-a\nfn verifies_a() {}\n");

    let report = affected(root, &[PathBuf::from("src/lib.rs")]).unwrap();

    assert_eq!(
        report.impls,
        vec![AffectedNode {
            id: "impl-a".to_string(),
            file: PathBuf::from("src/lib.rs"),
            line: 1,
        }]
    );
    assert_eq!(
        report.requirements,
        vec![AffectedNode {
            id: "FR-A".to_string(),
            file: PathBuf::from("docs/spec.md"),
            line: 1,
        }]
    );
    assert_eq!(
        report.tests,
        vec![AffectedTest {
            id: "test-a".to_string(),
            file: PathBuf::from("tests/a_test.rs"),
            line: 1,
            function: Some("verifies_a".to_string()),
        }]
    );
    assert!(
        report
            .commands
            .iter()
            .any(|c| c.contains("cargo test --test a_test")),
        "unexpected commands: {:?}",
        report.commands
    );
}

#[test]
fn test_affected_legacy_same_id_fallback() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // No @rel lines: impl, req, and test share the same id FR-A.
    write_tag_file(root, "docs/spec.md", "@req:FR-A:MUST\n");
    write_tag_file(root, "src/lib.rs", "// @impl:FR-A\n");
    write_tag_file(root, "tests/a_test.rs", "// @test:FR-A\nfn verifies_a() {}\n");

    let report = affected(root, &[PathBuf::from("src/lib.rs")]).unwrap();

    assert_eq!(report.impls.len(), 1);
    assert_eq!(report.impls[0].id, "FR-A");
    assert_eq!(report.impls[0].file, PathBuf::from("src/lib.rs"));

    assert_eq!(report.requirements.len(), 1);
    assert_eq!(report.requirements[0].id, "FR-A");
    assert_eq!(report.requirements[0].file, PathBuf::from("docs/spec.md"));

    assert_eq!(report.tests.len(), 1);
    assert_eq!(report.tests[0].id, "FR-A");
    assert_eq!(report.tests[0].file, PathBuf::from("tests/a_test.rs"));
    assert_eq!(report.tests[0].function, Some("verifies_a".to_string()));

    assert!(
        report
            .commands
            .iter()
            .any(|c| c.contains("cargo test --test a_test")),
        "unexpected commands: {:?}",
        report.commands
    );
}

#[test]
fn test_affected_no_change_returns_empty() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    write_tag_file(root, "docs/spec.md", "@req:FR-A:MUST\n");
    write_tag_file(root, "src/lib.rs", "// @impl:impl-a\n");

    let report = affected(root, &[PathBuf::from("other.rs")]).unwrap();
    assert_empty(&report);
}

#[test]
fn test_affected_absolute_path_is_stripped() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    write_tag_file(root, "src/lib.rs", "// @impl:impl-a\n");

    let report = affected(root, &[root.join("src/lib.rs")]).unwrap();
    assert_eq!(report.impls.len(), 1);
    assert_eq!(report.impls[0].id, "impl-a");
    assert_eq!(report.impls[0].file, PathBuf::from("src/lib.rs"));
    assert_eq!(report.requirements.len(), 0);
    assert_eq!(report.tests.len(), 0);
    assert_eq!(report.commands.len(), 0);
}
