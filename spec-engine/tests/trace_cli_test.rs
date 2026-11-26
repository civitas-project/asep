// @test:IN-01-REPORT
use asep::observability::trace_key;
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

fn trace_line(trace_id: &str, seq: u64, task_id: &str, event: &str) -> String {
    serde_json::to_string(&serde_json::json!({
        "schema_version": 1,
        "trace_id": trace_id,
        "parent_id": null,
        "task_id": task_id,
        "seq": seq,
        "ts_wall": "2026-11-24T00:00:00Z",
        "event": event,
        "duration_ms": null,
        "cost": null,
        "missing": [],
        "detail": null,
    }))
    .unwrap()
}

// @test:IN-01-REPORT
#[test]
fn test_trace_report_groups_counts_and_reconciles_verification() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    let trace_dir = root.join(".spec-engine").join("trace");
    fs::create_dir_all(&trace_dir).unwrap();

    fs::write(
        trace_dir.join(format!("{}.ndjson", trace_key("trace-abc"))),
        [
            trace_line("trace-abc", 0, "task-1", "run"),
            trace_line("trace-abc", 1, "task-1", "run"),
            trace_line("trace-abc", 2, "task-2", "finish"),
        ]
        .join("\n"),
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
    assert!(text.contains("run: 2"), "report must show event counts");
    assert!(
        text.contains("unknown"),
        "absent verification.json must report unknown"
    );
}

// @test:IN-01-REPORT
#[test]
fn test_trace_report_does_not_leak_into_run10_segments() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    let trace_dir = root.join(".spec-engine").join("trace");
    fs::create_dir_all(&trace_dir).unwrap();

    let run1_key = asep::observability::trace_key("run1");
    let run10_key = asep::observability::trace_key("run10");

    fs::write(
        trace_dir.join(format!("{run1_key}.ndjson")),
        [trace_line("run1", 0, "task-a", "run"), trace_line("run1", 1, "task-a", "finish")]
            .join("\n"),
    )
    .unwrap();
    fs::write(
        trace_dir.join(format!("{run1_key}.3.ndjson")),
        trace_line("run1", 2, "task-a", "recover"),
    )
    .unwrap();
    fs::write(
        trace_dir.join(format!("{run10_key}.ndjson")),
        [trace_line("run10", 0, "task-b", "run"), trace_line("run10", 1, "task-b", "finish")]
            .join("\n"),
    )
    .unwrap();

    let mut cmd = asep_bin();
    cmd.current_dir(root).arg("trace").arg("report").arg("run1");
    let output = cmd.assert().success().get_output().stdout.clone();
    let text = String::from_utf8(output).expect("report output must be UTF-8");
    assert!(text.contains("task-a"), "run1 report must group run1 tasks");
    assert!(text.contains("run: 1"), "run1 report must count run1 run event");
    assert!(text.contains("finish: 1"), "run1 report must count run1 finish event");
    assert!(text.contains("recover: 1"), "run1 report must include rotated segment");
    assert!(!text.contains("task-b"), "run1 report must not leak run10 task");
    assert!(!text.contains("run10"), "run1 report must not name run10");
}
