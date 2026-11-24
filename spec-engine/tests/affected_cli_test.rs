use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use std::path::Path;
use tempfile::tempdir;

fn write_tag_file(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&path, content).unwrap();
}

fn asep_bin() -> Command {
    Command::cargo_bin("asep").expect("asep binary")
}

// @test:FR-CLI-AFFECTED-01
#[test]
fn test_cli_affected_human_report() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();

    write_tag_file(
        root,
        "docs/spec.md",
        "@req:FR-A:MUST\n@rel:satisfies:impl-a->FR-A\n@rel:verifies:test-a->FR-A \"note\"\n",
    );
    write_tag_file(root, "src/lib.rs", "// @impl:impl-a\n");
    write_tag_file(
        root,
        "tests/a_test.rs",
        "// @test:test-a\nfn verifies_a() {}\n",
    );

    let mut cmd = asep_bin();
    cmd.arg("affected")
        .arg("src/lib.rs")
        .arg("--root")
        .arg(root);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("a_test.rs"))
        .stdout(predicate::str::contains("cargo test --test a_test"));
}

// @test:FR-CLI-AFFECTED-02
#[test]
fn test_cli_affected_json_report() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();

    // Legacy same-id fallback: @impl/@req/@test share id FR-A without @rel edges.
    write_tag_file(root, "docs/spec.md", "@req:FR-A:MUST\n");
    write_tag_file(root, "src/lib.rs", "// @impl:FR-A\n");
    write_tag_file(
        root,
        "tests/a_test.rs",
        "// @test:FR-A\nfn verifies_a() {}\n",
    );

    let mut cmd = asep_bin();
    cmd.arg("affected")
        .arg("src/lib.rs")
        .arg("--root")
        .arg(root)
        .arg("--json");
    let output = cmd.assert().success().get_output().stdout.clone();
    let json = String::from_utf8(output).expect("affected JSON output must be UTF-8");
    let report: Value = serde_json::from_str(&json).expect("affected report must be valid JSON");
    let tests = report
        .get("tests")
        .and_then(|t| t.as_array())
        .expect("affected JSON must contain a tests array");
    assert!(!tests.is_empty(), "affected tests must be non-empty");
}

// @test:FR-CLI-AFFECTED-03
#[test]
fn test_cli_affected_reads_paths_from_stdin() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();

    write_tag_file(root, "docs/spec.md", "@req:FR-A:MUST\n");
    write_tag_file(root, "src/lib.rs", "// @impl:FR-A\n");
    write_tag_file(
        root,
        "tests/a_test.rs",
        "// @test:FR-A\nfn verifies_a() {}\n",
    );

    let mut cmd = asep_bin();
    cmd.arg("affected")
        .arg("--stdin")
        .arg("--root")
        .arg(root)
        .write_stdin("\nsrc/lib.rs\n\n");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("a_test.rs"))
        .stdout(predicate::str::contains("cargo test --test a_test"));
}
