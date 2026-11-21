use asep::harness::e2e::{new_run, seed_from_run_id};
use serde_json::Value;
use std::path::Path;

fn parse_manifest(path: &Path) -> Value {
    let content = std::fs::read_to_string(path).expect("manifest.json should exist");
    serde_json::from_str(&content).expect("manifest.json should be valid JSON")
}

#[test]
fn test_new_run_writes_manifest() {
    let root = tempfile::tempdir().expect("failed to create temp dir");
    let run_id = "test-run";
    let run_dir = new_run(root.path(), run_id).expect("new_run should succeed");

    assert!(run_dir.is_dir(), "run directory should exist");

    let manifest_path = run_dir.join("manifest.json");
    assert!(manifest_path.is_file(), "manifest.json should exist");

    let manifest = parse_manifest(&manifest_path);
    assert_eq!(
        manifest.get("schema_version").and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        manifest.get("run_id").and_then(Value::as_str),
        Some(run_id)
    );
    assert!(manifest.get("seed").and_then(Value::as_u64).is_some());
    assert!(
        manifest.get("rustc_version").and_then(Value::as_str).is_some()
    );
    assert!(
        manifest.get("asep_commit").and_then(Value::as_str).is_some()
    );
    assert!(
        manifest.get("asep_binary").and_then(Value::as_str).is_some()
    );
    assert!(
        manifest.get("created_at").and_then(Value::as_str).is_some()
    );

    // Seed is deterministic for the same run-id.
    let run_dir_again = new_run(root.path(), run_id).expect("new_run should succeed");
    assert_eq!(run_dir, run_dir_again);
    let manifest_again = parse_manifest(&run_dir_again.join("manifest.json"));
    assert_eq!(
        manifest.get("seed"),
        manifest_again.get("seed"),
        "seed must be deterministic per run-id"
    );
}

#[test]
fn test_new_run_rejects_empty_id() {
    let root = tempfile::tempdir().expect("failed to create temp dir");
    let err = new_run(root.path(), "").unwrap_err();
    assert!(!err.is_empty(), "empty run_id should yield a clear error");
}

#[test]
fn test_seed_is_deterministic_and_matches_legacy_values() {
    assert_eq!(seed_from_run_id("dry-run-1"), 4044884631);
    assert_eq!(seed_from_run_id("test-run"), 883735528);
    assert_eq!(seed_from_run_id("test-run-1"), 1078660607);
}

#[test]
fn test_seed_written_to_manifest() {
    let root = tempfile::tempdir().expect("failed to create temp dir");
    let run_id = "test-run";
    let run_dir = new_run(root.path(), run_id).expect("new_run should succeed");
    let manifest_path = run_dir.join("manifest.json");
    let manifest: Value = serde_json::from_str(
        &std::fs::read_to_string(&manifest_path).expect("manifest.json should exist"),
    )
    .expect("manifest.json should be valid JSON");
    assert_eq!(manifest.get("seed").and_then(Value::as_u64), Some(883735528));
}
