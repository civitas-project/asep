// @test:IN-01-REPORT
use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

fn asep_bin() -> Command {
    Command::cargo_bin("asep").expect("asep binary")
}

// @test:IN-01-REPORT
#[test]
fn test_trace_status_empty_project_reports_disabled_telemetry() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();

    let mut cmd = asep_bin();
    cmd.current_dir(root).arg("trace").arg("status");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("disabled"))
        .stdout(predicate::str::contains("trace dir"))
        .stdout(predicate::str::contains("segments"));
}

// @test:IN-01-REPORT
#[test]
fn test_trace_report_missing_trace_exits_zero_with_unknown_verification() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();

    let mut cmd = asep_bin();
    cmd.current_dir(root)
        .arg("trace")
        .arg("report")
        .arg("trace-does-not-exist");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("trace-does-not-exist"))
        .stdout(predicate::str::contains("unknown"));
}

// @test:IN-01-REPORT
#[test]
fn test_trace_purge_missing_trace_exits_zero() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();

    let mut cmd = asep_bin();
    cmd.current_dir(root)
        .arg("trace")
        .arg("purge")
        .arg("trace-does-not-exist");
    cmd.assert().success();
}

// @test:IN-01-REPORT
#[test]
fn test_trace_report_groups_counts_and_reconciles_verification() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    let trace_dir = root.join(".spec-engine").join("trace");
    fs::create_dir_all(&trace_dir).unwrap();

    fs::write(
        trace_dir.join("trace-abc.ndjson"),
        r#"{"task_id":"task-1","event":"run"}
{"task_id":"task-1","event":"run"}
{"task_id":"task-2","event":"finish"}"#,
    )
    .unwrap();

    let mut cmd = asep_bin();
    cmd.current_dir(root)
        .arg("trace")
        .arg("report")
        .arg("trace-abc");
    let output = cmd.assert().success().get_output().stdout.clone();
    let text = String::from_utf8(output).expect("report output must be UTF-8");
    assert!(text.contains("task-1"), "report must group by task_id");
    assert!(text.contains("task-2"), "report must group by task_id");
    assert!(text.contains("run"), "report must group by event");
    assert!(text.contains("finish"), "report must group by event");
    assert!(text.contains("2"), "report must show event counts");
    assert!(
        text.contains("unknown"),
        "absent verification.json must report unknown"
    );
}
