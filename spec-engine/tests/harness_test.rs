//! Integration tests for the ASEP e2e harness scaffold.

use serde_json::Value;
use std::env;
use std::path::{Path, PathBuf};
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

const REQUIRED_ARTIFACTS: &[&str] = &[
    "manifest.json",
    "01-prd.md", "01-judge.json",
    "02-tdd.md", "02-judge.json",
    "03-tests.diff", "03-red.log", "03-hash.txt",
    "04-impl.diff",
    "05-findings.md", "05-loom.log",
    "06-optimization.md",
    "07-judge.json", "07-check.log", "07-ci.log",
];

fn verifier_script(root: &Path) -> PathBuf {
    root.join("harness/e2e/scripts/verify-artifacts.sh")
}

fn run_verifier(root: &Path, run_dir: &Path) -> std::process::Output {
    Command::new("bash")
        .arg(verifier_script(root))
        .arg(run_dir)
        .current_dir(root)
        .output()
        .expect("verify-artifacts.sh should run")
}

fn read_verify_json(run_dir: &Path) -> Value {
    let text = std::fs::read_to_string(run_dir.join("verify.json"))
        .expect("verify.json written by verifier");
    serde_json::from_str(&text).expect("verify.json parses")
}

fn write_artifact(run_dir: &Path, name: &str, content: &str) {
    std::fs::write(run_dir.join(name), content).expect("artifact written");
}

fn fake_run_dir(tmp: &TempDir, run_id: &str, omit: Option<&str>) -> PathBuf {
    let run_dir = tmp.path().join(run_id);
    std::fs::create_dir_all(&run_dir).expect("run dir created");

    let manifest = serde_json::json!({
        "schema_version": 1,
        "run_id": run_id,
        "seed": 1,
        "rustc_version": "rustc 1.80.0 (stub)",
        "asep_binary": "/no/such/asep-binary",
        "created_at": "2026-11-19T00:00:00Z",
    });
    write_artifact(&run_dir, "manifest.json", &manifest.to_string());

    for name in REQUIRED_ARTIFACTS.iter().copied() {
        if omit != Some(name) {
            write_artifact(&run_dir, name, "stub artifact\n");
        }
    }
    write_artifact(&run_dir, "03-tests-post-tdd.diff", "post-tdd diff stub\n");
    run_dir
}

#[test]
fn test_verifier_catches_hash_drift() {
    let root = workspace_root();
    let tmp = TempDir::new().expect("temp dir");
    let run_dir = fake_run_dir(&tmp, "hash-drift-run", None);

    std::fs::create_dir_all(run_dir.join("tests")).expect("tests dir created");
    write_artifact(
        &run_dir,
        "tests/ring_buffer_test.rs",
        "#[test]\nfn placeholder() {\n    assert!(true);\n}\n",
    );
    // 03-hash.txt does not match the current test file.
    write_artifact(
        &run_dir,
        "03-hash.txt",
        "0000000000000000000000000000000000000000000000000000000000000000  tests/ring_buffer_test.rs\n",
    );

    let status = run_verifier(&root, &run_dir);
    assert_eq!(
        status.status.code(),
        Some(1),
        "verifier must exit 1 on hash drift"
    );

    let verify = read_verify_json(&run_dir);
    assert_eq!(
        verify.get("pass").and_then(Value::as_bool),
        Some(false),
        "pass must be false on hash drift"
    );
    assert_eq!(
        verify.get("hash_lock_ok").and_then(Value::as_bool),
        Some(false),
        "hash_lock_ok must be false on hash drift"
    );
}

#[test]
fn test_verifier_catches_missing_artifact() {
    let root = workspace_root();
    let tmp = TempDir::new().expect("temp dir");
    let run_dir = fake_run_dir(&tmp, "missing-artifact-run", Some("07-judge.json"));

    let status = run_verifier(&root, &run_dir);
    assert_eq!(
        status.status.code(),
        Some(1),
        "verifier must exit 1 on missing artifact"
    );

    let verify = read_verify_json(&run_dir);
    assert_eq!(
        verify.get("pass").and_then(Value::as_bool),
        Some(false),
        "pass must be false when an artifact is missing"
    );
    let artifacts = verify
        .get("artifacts")
        .and_then(Value::as_object)
        .expect("artifacts object");
    assert_eq!(
        artifacts.get("07-judge.json").and_then(Value::as_bool),
        Some(false),
        "artifacts.07-judge.json must be false when missing"
    );
    let reasons = verify
        .get("reasons")
        .and_then(Value::as_array)
        .expect("reasons array");
    assert!(
        !reasons.is_empty(),
        "reasons must list the missing artifact"
    );
}

const STRESS_FIXTURE: &str = "harness/e2e/fixtures/stress_test.rs";

fn stress_fixture_source() -> String {
    let root = workspace_root();
    let path = root.join(STRESS_FIXTURE);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("stress fixture {STRESS_FIXTURE} must exist: {e}"))
}

#[test]
fn test_stress_fixture_has_timeout_and_iterations() {
    let text = stress_fixture_source();
    assert!(
        text.contains("100_000"),
        "fixture must fix 100_000 operations per configuration"
    );
    assert!(
        text.contains("TIMEOUT_SECS") && text.contains("30"),
        "fixture must fix a 30-second timeout constant"
    );
    assert!(
        text.contains("manifest.json"),
        "fixture must read its seed from manifest.json"
    );
    assert!(
        text.contains("yield_now()"),
        "fixture must hook thread::yield_now() at seed-given points"
    );
}

#[test]
fn test_stress_fixture_has_no_unsafe() {
    let text = stress_fixture_source();
    assert!(!text.contains("unsafe"), "fixture must contain no `unsafe`");
}
