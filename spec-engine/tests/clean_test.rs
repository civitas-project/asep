use asep::clean::{CleanAction, measure_dir, plan, read_baseline, threshold_bytes, write_baseline};
use std::fs;
use tempfile::tempdir;

const GIB: u64 = 1024 * 1024 * 1024;

#[test]
fn test_threshold_formula_and_clamps() {
    assert_eq!(threshold_bytes(0), 4 * GIB);
    assert_eq!(threshold_bytes(8 * GIB), 14 * GIB);
    assert_eq!(threshold_bytes(100 * GIB), 16 * GIB);
}

#[test]
fn test_measure_dir_counts_files_recursively() {
    let dir = tempdir().expect("tempdir created");
    let root = dir.path();

    fs::create_dir_all(root.join("a").join("b")).expect("create nested dirs");
    fs::write(root.join("a").join("one.txt"), b"12345").expect("write one.txt");
    fs::write(root.join("a").join("b").join("two.txt"), b"67890").expect("write two.txt");
    fs::write(root.join("top.txt"), b"abc").expect("write top.txt");

    assert_eq!(measure_dir(root), 5 + 5 + 3);
}

#[test]
fn test_plan_no_action_below_threshold() {
    let dir = tempdir().expect("tempdir created");
    let target = dir.path();
    fs::write(target.join("small.txt"), b"hello").expect("write small.txt");

    let report = plan(target, 1024);
    assert_eq!(report.action, CleanAction::None);
    assert_eq!(report.before_bytes, 5);
    assert_eq!(report.after_bytes, 5);
    assert_eq!(report.removed_bytes, 0);
}

#[test]
fn test_plan_clean_package_above_threshold() {
    let dir = tempdir().expect("tempdir created");
    let target = dir.path();
    fs::write(target.join("big.txt"), b"0".repeat(100)).expect("write big.txt");

    let report = plan(target, 10);
    assert_eq!(report.action, CleanAction::CleanPackage);
    assert_eq!(report.before_bytes, 100);
    assert_eq!(report.after_bytes, 0);
    assert_eq!(report.removed_bytes, 100);
}

#[test]
fn test_baseline_roundtrip() {
    let dir = tempdir().expect("tempdir created");
    let root = dir.path();

    assert_eq!(read_baseline(root), None);

    write_baseline(root, 7 * GIB).expect("write baseline");
    assert_eq!(read_baseline(root), Some(7 * GIB));
}
