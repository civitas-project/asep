use asep::agent_protocol::hash_file;
use asep::config::settings::{load_config, TelemetryConfig};
use asep::observability::{
    CostMetadata, CostSource, FileChangeTracker, TraceCollector, TraceEvent, TraceReport,
    trace_key,
};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

fn telemetry_config() -> TelemetryConfig {
    TelemetryConfig {
        enabled: true,
        allowlist: vec![
            "schema_version".to_string(),
            "trace_id".to_string(),
            "parent_id".to_string(),
            "task_id".to_string(),
            "seq".to_string(),
            "ts_wall".to_string(),
            "event".to_string(),
            "duration_ms".to_string(),
            "cost".to_string(),
            "missing".to_string(),
        ],
        redact_patterns: vec!["SECRET".to_string()],
        ttl_secs: 86_400,
        max_bytes: 10_485_760,
        max_buffer_events: 1000,
        dir: "trace".to_string(),
    }
}

fn write_config_toml(root: &Path) {
    fs::create_dir_all(root.join(".spec-engine")).unwrap();
    fs::write(
        root.join(".spec-engine").join("config.toml"),
        "[telemetry]\nenabled = true\nallowlist = [\"schema_version\", \"trace_id\", \"parent_id\", \"task_id\", \"seq\", \"ts_wall\", \"event\", \"duration_ms\", \"cost\", \"missing\", \"detail\"]\nredact_patterns = [\"SECRET\"]\nttl_secs = 86400\nmax_bytes = 10485760\ndir = \"trace\"\n",
    )
    .unwrap();
}

fn event(
    trace_id: &str,
    seq: u64,
    task_id: &str,
    event_name: &str,
    parent: Option<&str>,
    cost: Option<CostMetadata>,
) -> TraceEvent {
    TraceEvent {
        schema_version: 1,
        trace_id: trace_id.to_string(),
        parent_id: parent.map(|text| text.to_string()),
        task_id: task_id.to_string(),
        seq,
        ts_wall: "2026-11-24T00:00:00Z".to_string(),
        event: event_name.to_string(),
        duration_ms: Some(10),
        cost,
        missing: Vec::new(),
        detail: None,
    }
}

fn cost(value: Option<f64>, source: CostSource, missing: Vec<String>) -> Option<CostMetadata> {
    Some(CostMetadata {
        value,
        currency: Some("usd".to_string()),
        source,
        missing,
    })
}

#[test]
fn observability_writes_redacted_ndjson_and_report_from_events() {
    let root = tempdir().unwrap();
    write_config_toml(root.path());
    let config_dir = root.path().join(".spec-engine");
    let config = load_config(config_dir.as_path());
    assert_eq!(config.telemetry.enabled, true);

    let mut collector = TraceCollector::new(root.path(), &config.telemetry);

    collector
        .record(event(
            "trace-int",
            0,
            "task-1",
            "run",
            None,
            cost(Some(1.5), CostSource::Measured, Vec::new()),
        ))
        .unwrap();
    collector
        .record(event(
            "trace-int",
            1,
            "task-1",
            "estimate",
            None,
            cost(Some(0.5), CostSource::Estimated, Vec::new()),
        ))
        .unwrap();
    collector
        .record(event(
            "trace-int",
            2,
            "task-2",
            "unavailable-op",
            None,
            cost(None, CostSource::Unavailable, vec!["fee".to_string()]),
        ))
        .unwrap();
    let mut nested = event(
        "trace-int",
        3,
        "task-2",
        "nested call",
        Some("parent-1"),
        None,
    );
    collector.record(nested).unwrap();
    collector
        .record(event("trace-int", 4, "task-retry", "retry", None, None))
        .unwrap();
    collector
        .record(event("trace-int", 5, "task-retry", "retry", None, None))
        .unwrap();
    collector
        .record(event("trace-int", 6, "task-3", "resume", None, None))
        .unwrap();
    let mut secret = event("trace-int", 7, "SECRET-token", "leak", None, None);
    collector.record(secret).unwrap();
    let mut failed = event("trace-int", 8, "task-4", "observation", None, None);
    failed.detail = Some(serde_json::json!({"ok": false, "reason": "task fail"}));
    collector.record(failed).unwrap();
    collector.flush("trace-int").unwrap();

    let path = root.path().join("trace").join(format!("{}.ndjson", trace_key("trace-int")));
    let content = fs::read_to_string(&path).unwrap();
    assert!(!content.contains("SECRET"));
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.len(), 9);

    let unavailable_line = lines
        .iter()
        .find(|line| line.contains("\"event\":\"unavailable-op\""))
        .unwrap();
    let event_back: TraceEvent = serde_json::from_str(unavailable_line).unwrap();
    let cost_back = event_back.cost.unwrap();
    assert_eq!(cost_back.value, None);
    assert_eq!(cost_back.source, CostSource::Unavailable);
    assert_eq!(cost_back.missing, vec!["fee".to_string()]);

    let parsed: Vec<TraceEvent> = lines
        .iter()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let cost_count = parsed
        .iter()
        .filter(|event| event.cost.as_ref().is_some_and(|cost| cost.value.is_some()))
        .count();

    let verification = serde_json::json!({
        "status": "partial",
        "skipped": ["slow"],
    });
    let report = TraceReport::from_events(&parsed, Some(&verification), "commit-abc", "on");
    assert_eq!(report.baseline_commit, "commit-abc");
    assert_eq!(report.event_coverage, "on");
    assert_eq!(report.metrics.len(), 9);
    for metric in &report.metrics {
        assert!(metric.denominator.is_some());
    }
    let value = serde_json::to_value(&report).unwrap();
    let object = value.as_object().unwrap();
    let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
    keys.sort();
    assert_eq!(
        keys,
        vec![
            "baseline_commit",
            "event_coverage",
            "failures",
            "metrics",
            "skipped"
        ]
    );
    assert_eq!(report.skipped, vec!["slow".to_string()]);
    assert_eq!(
        report.failures,
        vec!["observation failed: task=task-4".to_string()]
    );
    let cost_metric = report
        .metrics
        .iter()
        .find(|metric| metric.name == "cost_per_verified_result")
        .unwrap();
    assert_eq!(cost_metric.denominator, Some(cost_count as u64));
}

fn write_sampled_config_toml(root: &Path) {
    fs::create_dir_all(root.join(".spec-engine")).unwrap();
    fs::write(
        root.join(".spec-engine").join("config.toml"),
        "[telemetry]\nenabled = true\nallowlist = [\"schema_version\", \"trace_id\", \"parent_id\", \"task_id\", \"seq\", \"ts_wall\", \"event\", \"duration_ms\", \"cost\", \"missing\", \"detail\"]\nredact_patterns = []\nttl_secs = 86400\nmax_bytes = 10485760\ndir = \"trace\"\n",
    )
    .unwrap();
}

// hash_file in spec-engine/src/agent_protocol/mod.rs returns a plain hex
// digest (no "sha256:" prefix); observability.rs is out of scope for this
// task, so the tests below assert equality with hash_file() and 64-char hex.

#[test]
fn file_change_evidence_is_exact_and_survives_sampling() {
    let root = tempdir().unwrap();
    write_sampled_config_toml(root.path());
    let config_dir = root.path().join(".spec-engine");
    let config = load_config(config_dir.as_path());
    assert_eq!(config.telemetry.enabled, true);

    let a = root.path().join("a.txt");
    let b = root.path().join("b.txt");
    fs::write(&a, "alpha").unwrap();
    fs::write(&b, "bravo").unwrap();

    let mut tracker = FileChangeTracker::new(
        root.path(),
        vec![PathBuf::from("a.txt"), PathBuf::from("b.txt")],
    );
    tracker.baseline().unwrap();
    let mut collector = TraceCollector::new(root.path(), &config.telemetry);

    let count = tracker
        .checkpoint(&mut collector, "task-1", "span-1")
        .unwrap();
    assert_eq!(count, 0);

    let expected_before = hash_file(&a).unwrap();
    fs::write(&a, "alpha-changed").unwrap();
    let expected_after = hash_file(&a).unwrap();
    let count = tracker
        .checkpoint(&mut collector, "task-1", "span-1")
        .unwrap();
    assert_eq!(count, 1);
    collector.flush("task-1").unwrap();

    let path = root.path().join("trace").join(format!("{}.ndjson", trace_key("task-1")));
    let content = fs::read_to_string(&path).unwrap();
    let lines: Vec<Value> = content
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let changes: Vec<&Value> = lines
        .iter()
        .filter(|value| value.get("event").and_then(|v| v.as_str()) == Some("file_change"))
        .collect();
    assert_eq!(changes.len(), 1);
    let detail = changes[0]
        .get("detail")
        .and_then(|v| v.as_object())
        .unwrap();
    assert_eq!(detail.get("path").and_then(|v| v.as_str()), Some("a.txt"));
    let before = detail.get("before").and_then(|v| v.as_str()).unwrap();
    let after = detail.get("after").and_then(|v| v.as_str()).unwrap();
    assert_eq!(before, expected_before.as_str());
    assert_eq!(after, expected_after.as_str());
    assert_ne!(before, after);
    assert_eq!(before.len(), 64);
    assert_eq!(after.len(), 64);
    assert_eq!(detail.get("span").and_then(|v| v.as_str()), Some("span-1"));
    let mentions_b = lines
        .iter()
        .any(|value| value.to_string().contains("b.txt"));
    assert!(!mentions_b);
}

#[test]
fn intent_precedes_change_and_coverage_is_reported() {
    let root = tempdir().unwrap();
    write_sampled_config_toml(root.path());
    let config_dir = root.path().join(".spec-engine");
    let config = load_config(config_dir.as_path());
    assert_eq!(config.telemetry.enabled, true);

    let a = root.path().join("a.txt");
    fs::write(&a, "alpha").unwrap();

    let mut tracker = FileChangeTracker::new(root.path(), vec![PathBuf::from("a.txt")]);
    tracker.baseline().unwrap();
    let mut collector = TraceCollector::new(root.path(), &config.telemetry);

    tracker
        .intent(&mut collector, Path::new("a.txt"), "task-x")
        .unwrap();
    fs::write(&a, "alpha-changed").unwrap();
    tracker
        .checkpoint(&mut collector, "task-x", "span-1")
        .unwrap();
    collector.flush("task-x").unwrap();

    let path = root.path().join("trace").join(format!("{}.ndjson", trace_key("task-x")));
    let content = fs::read_to_string(&path).unwrap();
    let lines: Vec<Value> = content
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let intent_index = lines
        .iter()
        .position(|value| value.get("event").and_then(|v| v.as_str()) == Some("write_intent"))
        .unwrap();
    let change_index = lines
        .iter()
        .position(|value| value.get("event").and_then(|v| v.as_str()) == Some("file_change"))
        .unwrap();
    assert!(intent_index < change_index);
    let intent_count = lines
        .iter()
        .filter(|value| value.get("event").and_then(|v| v.as_str()) == Some("write_intent"))
        .count();
    let change_count = lines
        .iter()
        .filter(|value| value.get("event").and_then(|v| v.as_str()) == Some("file_change"))
        .count();
    assert_eq!(intent_count, 1);
    assert_eq!(change_count, 1);
    let intent_detail = lines[intent_index]
        .get("detail")
        .and_then(|v| v.as_object())
        .unwrap();
    let change_detail = lines[change_index]
        .get("detail")
        .and_then(|v| v.as_object())
        .unwrap();
    assert_eq!(
        intent_detail.get("before").and_then(|v| v.as_str()),
        change_detail.get("before").and_then(|v| v.as_str())
    );

    let parsed: Vec<TraceEvent> = lines
        .iter()
        .map(|value| serde_json::from_value(value.clone()).unwrap())
        .collect();
    let report = TraceReport::from_events(&parsed, None, "baseline", "off");
    assert_eq!(report.event_coverage, "off");
}
