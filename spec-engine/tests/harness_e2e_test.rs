use asep::harness::e2e::{
    new_run, prepare_vendor_dir, repo_root, seed_from_run_id, verify_preflight,
};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

const REQUIRED_ARTIFACT_NAMES: &[&str] = &[
    "manifest.json",
    "01-prd.md",
    "01-judge.json",
    "02-tdd.md",
    "02-judge.json",
    "03-tests.diff",
    "03-red.log",
    "03-hash.txt",
    "04-impl.diff",
    "05-findings.md",
    "05-loom.log",
    "06-optimization.md",
    "07-judge.json",
    "07-check.log",
    "07-ci.log",
];

fn write_test_file(run_dir: &Path) -> PathBuf {
    let dummy = run_dir.join("dummy");
    fs::create_dir_all(&dummy).expect("dummy dir created");
    let test_file = dummy.join("one_test.rs");
    fs::write(&test_file, "fn one_test() {}\n").expect("test file should be written");
    test_file
}

fn write_dummy_hash(run_dir: &Path) {
    fs::write(
        run_dir.join("03-hash.txt"),
        "0000000000000000000000000000000000000000000000000000000000000000 dummy\n",
    )
    .expect("03-hash.txt should be written");
}

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
    assert_eq!(manifest.get("run_id").and_then(Value::as_str), Some(run_id));
    assert!(manifest.get("seed").and_then(Value::as_u64).is_some());
    assert!(manifest
        .get("rustc_version")
        .and_then(Value::as_str)
        .is_some());
    assert!(manifest
        .get("asep_commit")
        .and_then(Value::as_str)
        .is_some());
    assert!(manifest
        .get("asep_binary")
        .and_then(Value::as_str)
        .is_some());
    assert!(manifest.get("created_at").and_then(Value::as_str).is_some());

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
    assert_eq!(
        manifest.get("seed").and_then(Value::as_u64),
        Some(883735528)
    );
}

#[test]
fn test_prepare_vendor_dir_creates_config_and_vendor() {
    let run_dir = tempfile::tempdir().expect("failed to create temp dir");
    let repo_root = repo_root();
    let vendor_src = repo_root.join("harness").join("e2e").join("vendor-loom");
    prepare_vendor_dir(run_dir.path(), &vendor_src).expect("prepare_vendor_dir should succeed");

    assert!(
        run_dir.path().join("vendor").join("loom").is_dir(),
        "vendored loom source should exist under run_dir/vendor/"
    );

    let config_path = run_dir.path().join(".cargo").join("config.toml");
    let config_text = fs::read_to_string(&config_path).expect("config.toml should exist");
    assert!(
        config_text.contains("vendored-sources"),
        "config.toml should name the vendored source"
    );
    assert!(
        config_text.contains(&format!(
            "directory = \"{}\"",
            run_dir.path().join("vendor").display()
        )),
        "config.toml should point directory at the absolute vendor path"
    );
}

#[test]
fn test_prepare_vendor_dir_refuses_existing() {
    let run_dir = tempfile::tempdir().expect("failed to create temp dir");
    let repo_root = repo_root();
    let vendor_src = repo_root.join("harness").join("e2e").join("vendor-loom");
    prepare_vendor_dir(run_dir.path(), &vendor_src).expect("first call should succeed");
    let err = prepare_vendor_dir(run_dir.path(), &vendor_src).unwrap_err();
    assert!(!err.is_empty(), "second call should refuse to overwrite");
}

#[test]
fn test_verify_detects_missing_artifacts_and_hash() {
    let run_dir = tempfile::tempdir().expect("failed to create temp dir");
    write_test_file(run_dir.path());
    write_dummy_hash(run_dir.path());

    let (artifacts, hash_lock_ok, reasons) = verify_preflight(run_dir.path());

    // Exactly the required artifact names, all missing except 03-hash.txt.
    assert_eq!(artifacts.len(), REQUIRED_ARTIFACT_NAMES.len());
    for name in REQUIRED_ARTIFACT_NAMES {
        let present = artifacts.get(*name).copied().unwrap_or(false);
        assert_eq!(
            present,
            *name == "03-hash.txt",
            "artifact '{name}' presence mismatch"
        );
    }

    // Dummy hash matches neither the aggregate nor any single test file.
    assert!(
        !hash_lock_ok,
        "hash lock should be unsatisfiable: {reasons:?}"
    );
    assert!(
        reasons
            .iter()
            .any(|reason| reason.starts_with("missing artifacts:")),
        "expected missing-artifacts reason, got {reasons:?}"
    );
    assert!(
        reasons
            .iter()
            .any(|reason| reason.starts_with("hash lock:")),
        "expected hash-lock reason, got {reasons:?}"
    );
}

#[test]
fn test_verify_controller_ruling_overrides_hash() {
    let run_dir = tempfile::tempdir().expect("failed to create temp dir");
    write_test_file(run_dir.path());
    write_dummy_hash(run_dir.path());
    fs::write(run_dir.path().join("controller-ruling.json"), "{}\n")
        .expect("controller-ruling.json should be written");

    let (_artifacts, hash_lock_ok, _reasons) = verify_preflight(run_dir.path());
    assert!(hash_lock_ok, "controller ruling should override hash lock");
}

#[test]
fn test_hash_lock_collects_tests_in_multiple_subdirectories() {
    use sha2::{Digest, Sha256};
    let run_dir = tempfile::tempdir().expect("failed to create temp dir");
    let dummy = run_dir.path().join("dummy");
    // Two test files in two DIFFERENT subdirectories of dummy/.
    let a_dir = dummy.join("mod_a");
    let b_dir = dummy.join("mod_b");
    fs::create_dir_all(&a_dir).expect("mod_a created");
    fs::create_dir_all(&b_dir).expect("mod_b created");
    let fa = a_dir.join("alpha_test.rs");
    let fb = b_dir.join("beta_test.rs");
    fs::write(&fa, "fn alpha() {}\n").expect("alpha written");
    fs::write(&fb, "fn beta() {}\n").expect("beta written");

    // Compute the exact aggregate hash the auditor should produce from BOTH files.
    let hex = |data: &[u8]| -> String {
        let mut h = Sha256::new();
        h.update(data);
        h.finalize().iter().map(|b| format!("{b:02x}")).collect()
    };
    let mut lines = vec![
        format!(
            "{}  {}",
            hex(fs::read(&fa).unwrap().as_slice()),
            fa.strip_prefix(run_dir.path()).unwrap().to_string_lossy()
        ),
        format!(
            "{}  {}",
            hex(fs::read(&fb).unwrap().as_slice()),
            fb.strip_prefix(run_dir.path()).unwrap().to_string_lossy()
        ),
    ];
    lines.sort();
    let mut listing = lines.join("\n");
    listing.push('\n');
    let aggregate = hex(listing.as_bytes());
    fs::write(
        run_dir.path().join("03-hash.txt"),
        format!("{aggregate}  all\n"),
    )
    .expect("03-hash.txt written");

    let (_artifacts, hash_lock_ok, reasons) = verify_preflight(run_dir.path());
    assert!(
        hash_lock_ok,
        "hash lock should cover test files in all subdirectories: {reasons:?}"
    );
}
