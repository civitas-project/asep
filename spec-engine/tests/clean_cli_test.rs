use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

fn asep_bin() -> Command {
    Command::cargo_bin("asep").expect("asep binary")
}

fn setup_target(root: &PathBuf) -> PathBuf {
    let target = root.join("target");
    fs::create_dir_all(target.join("deps")).unwrap();
    fs::write(target.join("big.rlib"), b"0".repeat(1000)).unwrap();
    fs::write(target.join("deps").join("lib.rlib"), b"1".repeat(1000)).unwrap();
    target
}

#[test]
fn test_clean_measure_writes_baseline() {
    let tmp = tempdir().unwrap();
    let root = tmp.path().to_path_buf();
    let target = setup_target(&root);

    let mut cmd = asep_bin();
    cmd.current_dir(&root)
        .arg("clean")
        .arg("--measure")
        .arg("--target")
        .arg(&target);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("baseline"))
        .stdout(predicate::str::contains("threshold"));

    assert!(
        root.join(".spec-engine").join("target-baseline.json").exists(),
        "baseline file must be written at <root>/.spec-engine/target-baseline.json"
    );
}

#[test]
fn test_clean_dry_run_does_not_delete() {
    let tmp = tempdir().unwrap();
    let root = tmp.path().to_path_buf();
    let target = setup_target(&root);
    assert!(target.join("big.rlib").exists());

    let mut cmd = asep_bin();
    cmd.current_dir(&root)
        .arg("clean")
        .arg("--max-gb")
        .arg("0.000001")
        .arg("--dry-run")
        .arg("--target")
        .arg(&target);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("CleanPackage"));

    assert!(target.join("big.rlib").exists(), "files must survive dry run");
    assert!(target.join("deps").join("lib.rlib").exists());
}

#[test]
fn test_clean_accepts_root_flag() {
    let tmp = tempdir().unwrap();
    let root = tmp.path().to_path_buf();
    let target = setup_target(&root);

    let mut cmd = asep_bin();
    cmd.arg("clean")
        .arg("--root")
        .arg(&root)
        .arg("--dry-run")
        .arg("--max-gb")
        .arg("8");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(
            target.to_string_lossy().as_ref(),
        ));
}

#[test]
fn test_clean_rejects_zero_max_gb_in_dry_run() {
    let tmp = tempdir().unwrap();
    let root = tmp.path().to_path_buf();
    setup_target(&root);

    let mut cmd = asep_bin();
    cmd.current_dir(&root)
        .arg("clean")
        .arg("--max-gb")
        .arg("0")
        .arg("--dry-run")
        .arg("--target")
        .arg(root.join("target"));

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("max-gb"));
}

#[test]
fn test_clean_rejects_zero_max_gb() {
    let tmp = tempdir().unwrap();
    let root = tmp.path().to_path_buf();
    setup_target(&root);

    let mut cmd = asep_bin();
    cmd.current_dir(&root)
        .arg("clean")
        .arg("--max-gb")
        .arg("0")
        .arg("--target")
        .arg(root.join("target"));

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("max-gb"));
}
