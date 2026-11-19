//! Integration tests for the ASEP e2e harness scaffold.

use serde_json::Value;
use std::env;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn workspace_root() -> std::path::PathBuf {
    Path::new(&env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("spec-engine has a workspace root")
        .to_path_buf()
}

fn git_short_commit(repo: &Path) -> String {
    String::from_utf8(
        Command::new("git")
            .args(["rev-parse", "--short", "HEAD"])
            .current_dir(repo)
            .output()
            .expect("git rev-parse --short HEAD")
            .stdout,
    )
    .expect("git output is utf-8")
    .trim()
    .to_string()
}

#[test]
fn test_new_run_creates_manifest() {
    let root = workspace_root();
    let script = root.join("harness/e2e/scripts/new-run.sh");
    let tmp = TempDir::new().expect("temp dir");
    let run_id = "test-run-1";

    let status = Command::new("bash")
        .arg(&script)
        .arg(run_id)
        .env("RUN_ROOT", tmp.path())
        .current_dir(&root)
        .status()
        .expect("new-run.sh executable");

    assert!(status.success(), "new-run.sh exited non-zero");

    let run_dir = tmp.path().join(run_id);
    assert!(run_dir.is_dir(), "run dir created at {}", run_dir.display());

    let manifest = run_dir.join("manifest.json");
    assert!(manifest.is_file(), "manifest.json written");

    let text = std::fs::read_to_string(&manifest).expect("read manifest.json");
    let value: Value = serde_json::from_str(&text).expect("manifest.json parses");

    assert_eq!(
        value.get("schema_version").and_then(Value::as_u64),
        Some(1),
        "schema_version == 1"
    );

    let seed = value
        .get("seed")
        .and_then(Value::as_u64)
        .expect("seed present");
    assert!(
        (1..=u32::MAX as u64).contains(&seed),
        "seed is non-empty 32-bit: {seed}"
    );

    let commit = value
        .get("asep_commit")
        .and_then(Value::as_str)
        .expect("asep_commit present");
    assert_eq!(
        commit,
        git_short_commit(&root),
        "asep_commit matches git rev-parse --short HEAD"
    );

    assert_eq!(
        value.get("run_id").and_then(Value::as_str),
        Some(run_id),
        "run_id echoed"
    );
    for key in ["rustc_version", "asep_binary", "created_at"] {
        assert!(
            value.get(key).and_then(Value::as_str).map_or(false, |s| !s.is_empty()),
            "{key} non-empty"
        );
    }
}
