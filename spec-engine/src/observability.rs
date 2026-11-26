// @impl:IN-01-CONTRACT
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use crate::agent_protocol::{hash_file, now_rfc3339};
use crate::config::settings::TelemetryConfig;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CostSource {
    Measured,
    Estimated,
    #[serde(rename = "provider_reported")]
    ProviderReported,
    Unavailable,
}

// @impl:IN-01-COST
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostMetadata {
    pub value: Option<f64>,
    pub currency: Option<String>,
    pub source: CostSource,
    pub missing: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEvent {
    pub schema_version: u8,
    pub trace_id: String,
    pub parent_id: Option<String>,
    pub task_id: String,
    pub seq: u64,
    pub ts_wall: String,
    pub event: String,
    pub duration_ms: Option<u128>,
    pub cost: Option<CostMetadata>,
    pub missing: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<serde_json::Value>,
}

// @impl:IN-01-METRIC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    pub name: String,
    pub value: Option<f64>,
    pub denominator: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceReport {
    pub baseline_commit: String,
    pub metrics: Vec<Metric>,
    pub skipped: Vec<String>,
    pub failures: Vec<String>,
    pub event_coverage: String,
}

fn rate(count: u64, total: u64) -> Option<f64> {
    (total > 0).then_some(count as f64 / total as f64)
}

impl TraceReport {
    // Derivation rules:
    // gate_pass_rate    = pass gate-result events / gate-result events (name is gate_pass/gate_fail or ends _pass/_fail)
    // defect_severity   = count(event name contains "defect")
    // reproducibility   = count(event name contains "repro") / total events
    // tool_call_count   = count(event name contains "tool")
    // token_count       = sum of numeric detail.tokens / detail.token_count; None when no event carries either
    // latency_ms        = sum of duration_ms from events that have one; None when none have it
    // cost_per_verified_result = sum of cost.value / events whose cost.value is Some; None when none
    // recovery_fidelity = count(event name contains "recover") / total events
    // overhead          = count(events with parent_id set) / total events
    // @impl:FR-AUD-18
    pub fn from_events(
        events: &[TraceEvent],
        verification: Option<&serde_json::Value>,
        baseline_commit: &str,
        event_coverage: &str,
    ) -> Self {
        let total = events.len() as u64;

        let is_gate_result = |name: &str| name.ends_with("_pass") || name.ends_with("_fail");
        let is_gate_pass = |name: &str| name.ends_with("_pass");
        let gate_events: Vec<&TraceEvent> = events
            .iter()
            .filter(|event| is_gate_result(&event.event))
            .collect();
        let gate_total = gate_events.len() as u64;
        let pass_count = gate_events
            .iter()
            .filter(|event| is_gate_pass(&event.event))
            .count() as u64;

        let defect_count = events
            .iter()
            .filter(|event| event.event.contains("defect"))
            .count() as u64;
        let repro_count = events
            .iter()
            .filter(|event| event.event.contains("repro"))
            .count() as u64;
        let tool_count = events
            .iter()
            .filter(|event| event.event.contains("tool"))
            .count() as u64;
        let recover_count = events
            .iter()
            .filter(|event| event.event.contains("recover"))
            .count() as u64;
        let nested_count = events
            .iter()
            .filter(|event| event.parent_id.is_some())
            .count() as u64;

        let gate_pass_rate = rate(pass_count, gate_total);
        let defect_severity = rate(defect_count, total);
        let reproducibility = rate(repro_count, total);
        let tool_call_count = (total > 0).then_some(tool_count as f64);

        let token_count: Option<f64> = events
            .iter()
            .filter_map(|event| {
                event.detail.as_ref().and_then(|detail| {
                    detail.get("tokens").or_else(|| detail.get("token_count")).and_then(|value| value.as_f64())
                })
            })
            .fold(None, |acc, value| Some(acc.unwrap_or(0.0) + value));

        let latency_ms: Option<f64> = events
            .iter()
            .filter_map(|event| event.duration_ms.map(|millis| millis as f64))
            .fold(None, |acc, value| Some(acc.unwrap_or(0.0) + value));

        let cost_denominator = events
            .iter()
            .filter(|event| event.cost.as_ref().and_then(|cost| cost.value).is_some())
            .count() as u64;
        let cost_sum: f64 = events
            .iter()
            .filter_map(|event| event.cost.as_ref().and_then(|cost| cost.value))
            .sum();
        let cost_per_verified_result =
            (cost_denominator > 0).then_some(cost_sum / cost_denominator as f64);
        let recovery_fidelity = rate(recover_count, total);
        let overhead = rate(nested_count, total);

        let metrics = vec![
            Metric {
                name: "gate_pass_rate".to_string(),
                value: gate_pass_rate,
                denominator: Some(gate_total),
            },
            Metric {
                name: "defect_severity".to_string(),
                value: defect_severity,
                denominator: Some(total),
            },
            Metric {
                name: "reproducibility".to_string(),
                value: reproducibility,
                denominator: Some(total),
            },
            Metric {
                name: "tool_call_count".to_string(),
                value: tool_call_count,
                denominator: Some(total),
            },
            Metric {
                name: "token_count".to_string(),
                value: token_count,
                denominator: Some(total),
            },
            Metric {
                name: "latency_ms".to_string(),
                value: latency_ms,
                denominator: Some(total),
            },
            Metric {
                name: "cost_per_verified_result".to_string(),
                value: cost_per_verified_result,
                denominator: Some(cost_denominator),
            },
            Metric {
                name: "recovery_fidelity".to_string(),
                value: recovery_fidelity,
                denominator: Some(total),
            },
            Metric {
                name: "overhead".to_string(),
                value: overhead,
                denominator: Some(total),
            },
        ];

        let failures: Vec<String> = events
            .iter()
            .filter_map(|event| {
                let failed_observation = event.event == "observation"
                    && event
                        .detail
                        .as_ref()
                        .and_then(|detail| detail.get("ok"))
                        .and_then(|ok| ok.as_bool())
                        == Some(false);
                failed_observation.then(|| format!("observation failed: task={}", event.task_id))
            })
            .collect();

        let skipped = verification
            .and_then(|value| value.get("skipped"))
            .and_then(|value| value.as_array())
            .map(|array| {
                array
                    .iter()
                    .filter_map(|item| item.as_str().map(|text| text.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        Self {
            baseline_commit: baseline_commit.to_string(),
            metrics,
            skipped,
            failures,
            event_coverage: event_coverage.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TraceSpan {
    pub trace_id: String,
    pub parent_id: Option<String>,
    pub task_id: String,
    pub event: String,
    pub ts_wall: String,
}

impl TraceSpan {
    pub fn new(
        trace_id: String,
        parent_id: Option<String>,
        task_id: String,
        event: String,
        ts_wall: String,
    ) -> Self {
        Self {
            trace_id,
            parent_id,
            task_id,
            event,
            ts_wall,
        }
    }
}

pub struct SpanGuard {
    span: TraceSpan,
    started: Instant,
}

impl SpanGuard {
    pub fn new(span: TraceSpan) -> Self {
        Self {
            span,
            started: Instant::now(),
        }
    }

    pub fn finish(self, seq: u64) -> TraceEvent {
        TraceEvent {
            schema_version: 1,
            trace_id: self.span.trace_id.clone(),
            parent_id: self.span.parent_id.clone(),
            task_id: self.span.task_id.clone(),
            seq,
            ts_wall: self.span.ts_wall.clone(),
            event: self.span.event.clone(),
            duration_ms: Some(self.started.elapsed().as_millis()),
            cost: None,
            missing: Vec::new(),
            detail: None,
        }
    }
}

impl Drop for SpanGuard {
    fn drop(&mut self) {
        // SpanGuard does not emit a TraceEvent on drop; the collector owns emission later.
    }
}

// @impl:IN-01-COLLECT
pub struct TraceCollector {
    target_dir: PathBuf,
    enabled: bool,
    allowlist: Vec<String>,
    redact_patterns: Vec<String>,
    ttl_secs: u64,
    max_bytes: u64,
    buffer_cap: usize,
    buffer: HashMap<String, Vec<TraceEvent>>,
}

impl TraceCollector {
    pub fn new(root: &Path, config: &TelemetryConfig) -> Self {
        let mut buffer_cap = config.max_buffer_events;
        if buffer_cap == 0 {
            buffer_cap = 1000;
        }
        Self {
            target_dir: root.join(&config.dir),
            enabled: config.enabled,
            allowlist: config.allowlist.clone(),
            redact_patterns: config.redact_patterns.clone(),
            ttl_secs: config.ttl_secs,
            max_bytes: config.max_bytes,
            buffer_cap,
            buffer: HashMap::new(),
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn record(&mut self, event: TraceEvent) -> Result<(), String> {
        if !self.enabled {
            return Ok(());
        }
        self.buffer
            .entry(event.trace_id.clone())
            .or_default()
            .push(event);
        self.auto_flush_if_needed()
    }

    // @impl:IN-01-INTENT
    pub fn record_evidence(&mut self, event: TraceEvent) -> Result<(), String> {
        self.buffer
            .entry(event.trace_id.clone())
            .or_default()
            .push(event);
        self.auto_flush_if_needed()
    }

    fn auto_flush_if_needed(&mut self) -> Result<(), String> {
        let total: usize = self.buffer.values().map(|v| v.len()).sum();
        if total >= self.buffer_cap {
            self.flush_all()?;
        }
        Ok(())
    }

    pub fn flush(&mut self, trace_id: &str) -> Result<(), String> {
        let events = match self.buffer.get(trace_id) {
            Some(v) if !v.is_empty() => v.clone(),
            _ => return Ok(()),
        };
        fs::create_dir_all(&self.target_dir).map_err(|e| e.to_string())?;
        let active = self.target_dir.join(format!("{trace_id}.ndjson"));
        let mut rotate = false;
        if active.exists() {
            rotate = fs::metadata(&active).map_err(|e| e.to_string())?.len() >= self.max_bytes;
        }
        if rotate {
            let secs = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_secs();
            let rotated = self.target_dir.join(format!("{trace_id}.{secs}.ndjson"));
            fs::rename(&active, &rotated).map_err(|e| e.to_string())?;
        }
        self.expire_rotated_segments(trace_id)?;
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&active)
            .map_err(|e| e.to_string())?;
        for event in &events {
            let line = serialize_event(event, &self.allowlist, &self.redact_patterns)?;
            writeln!(file, "{line}").map_err(|e| e.to_string())?;
        }
        file.set_permissions(PermissionsExt::from_mode(0o600))
            .map_err(|e| e.to_string())?;
        file.flush().map_err(|e| e.to_string())?;
        self.buffer.remove(trace_id);
        Ok(())
    }

    pub fn purge(&self, trace_id: &str) -> Result<(), String> {
        let active = self.target_dir.join(format!("{trace_id}.ndjson"));
        if active.exists() {
            fs::remove_file(&active).map_err(|e| e.to_string())?;
        }
        for path in self.rotated_segment_paths(trace_id)? {
            fs::remove_file(&path).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn flush_all(&mut self) -> Result<(), String> {
        let trace_ids: Vec<String> = self.buffer.keys().cloned().collect();
        for trace_id in trace_ids {
            self.flush(&trace_id)?;
        }
        Ok(())
    }

    pub fn close(&mut self) -> Result<(), String> {
        self.flush_all()
    }

    pub fn with<F, R>(root: &Path, config: &TelemetryConfig, f: F) -> Result<R, String>
    where
        F: FnOnce(&mut TraceCollector) -> Result<R, String>,
    {
        let mut c = Self::new(root, config);
        let res = f(&mut c)?;
        c.close()?;
        Ok(res)
    }
}

// @impl:IN-01-RETENTION
impl TraceCollector {
    fn rotated_segment_paths(&self, trace_id: &str) -> Result<Vec<PathBuf>, String> {
        if !self.target_dir.exists() {
            return Ok(Vec::new());
        }
        let mut out = Vec::new();
        let prefix = format!("{trace_id}.");
        let active = format!("{trace_id}.ndjson");
        let read_dir = fs::read_dir(&self.target_dir).map_err(|e| e.to_string())?;
        for entry in read_dir {
            let entry = entry.map_err(|e| e.to_string())?;
            let name = entry.file_name().to_string_lossy().to_string();
            if name == active {
                continue;
            }
            if name.starts_with(&prefix) && name.ends_with(".ndjson") {
                out.push(entry.path());
            }
        }
        Ok(out)
    }

    fn expire_rotated_segments(&self, trace_id: &str) -> Result<(), String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_secs();
        for path in self.rotated_segment_paths(trace_id)? {
            let metadata = fs::metadata(&path).map_err(|e| e.to_string())?;
            let mtime = metadata.modified().map_err(|e| e.to_string())?;
            let mtime_secs = mtime
                .duration_since(UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_secs();
            let age = now.saturating_sub(mtime_secs);
            if age >= self.ttl_secs {
                fs::remove_file(&path).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }
}

impl Drop for TraceCollector {
    fn drop(&mut self) {
        if let Err(e) = self.close() {
            eprintln!("observability: flush failed during drop: {e}");
        }
    }
}

// @impl:IN-01-FILE-EVIDENCE
pub struct FileChangeTracker {
    root: PathBuf,
    paths: Vec<PathBuf>,
    hashes: HashMap<PathBuf, String>,
    seq: u64,
}

impl FileChangeTracker {
    pub fn new(root: &Path, paths: Vec<PathBuf>) -> Self {
        Self {
            root: root.to_path_buf(),
            paths,
            hashes: HashMap::new(),
            seq: 0,
        }
    }

    pub fn baseline(&mut self) -> Result<(), String> {
        for path in &self.paths {
            let full = self.root.join(path);
            let hash = match hash_file(&full) {
                Ok(value) => value,
                Err(_) => "<missing>".to_string(),
            };
            self.hashes.insert(path.clone(), hash);
        }
        Ok(())
    }

    pub fn checkpoint(
        &mut self,
        collector: &mut TraceCollector,
        task_id: &str,
        span_label: &str,
    ) -> Result<usize, String> {
        let mut emitted = 0usize;
        for path in &self.paths {
            let full = self.root.join(path);
            let before = self
                .hashes
                .get(path)
                .cloned()
                .unwrap_or_else(|| "<missing>".to_string());
            let after = match hash_file(&full) {
                Ok(value) => value,
                Err(_) => "<missing>".to_string(),
            };
            if before == after {
                continue;
            }
            let seq = self.seq;
            self.seq += 1;
            let detail = serde_json::json!({
                "path": path.to_string_lossy(),
                "before": before,
                "after": after,
                "span": span_label,
            });
            let event = TraceEvent {
                schema_version: 1,
                trace_id: task_id.to_string(),
                parent_id: None,
                task_id: task_id.to_string(),
                seq,
                ts_wall: now_rfc3339(),
                event: "file_change".to_string(),
                duration_ms: None,
                cost: None,
                missing: Vec::new(),
                detail: Some(detail),
            };
            collector.record_evidence(event)?;
            self.hashes.insert(path.clone(), after);
            emitted += 1;
        }
        Ok(emitted)
    }

    pub fn intent(
        &mut self,
        collector: &mut TraceCollector,
        path: &Path,
        task_id: &str,
    ) -> Result<(), String> {
        let relative = path
            .strip_prefix(&self.root)
            .map(|part| part.to_path_buf())
            .unwrap_or(path.to_path_buf());
        let before = self
            .hashes
            .get(&relative)
            .cloned()
            .unwrap_or_else(|| "<missing>".to_string());
        let detail = serde_json::json!({
            "path": relative.to_string_lossy(),
            "before": before,
            "kind": "write_intent",
        });
        let seq = self.seq;
        self.seq += 1;
        let event = TraceEvent {
            schema_version: 1,
            trace_id: task_id.to_string(),
            parent_id: None,
            task_id: task_id.to_string(),
            seq,
            ts_wall: now_rfc3339(),
            event: "write_intent".to_string(),
            duration_ms: None,
            cost: None,
            missing: Vec::new(),
            detail: Some(detail),
        };
        collector.record_evidence(event)?;
        Ok(())
    }
}

// @impl:IN-01-PRIVACY
fn serialize_event(
    event: &TraceEvent,
    allowlist: &[String],
    redact_patterns: &[String],
) -> Result<String, String> {
    let value = serde_json::to_value(event).map_err(|e| e.to_string())?;
    let filtered = match value {
        serde_json::Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for (key, item) in map {
                if !allowlist.is_empty() && allowlist.contains(&key) {
                    out.insert(key, redact(item, redact_patterns));
                }
            }
            serde_json::Value::Object(out)
        }
        other => redact(other, redact_patterns),
    };
    serde_json::to_string(&filtered).map_err(|e| e.to_string())
}

fn redact(value: serde_json::Value, patterns: &[String]) -> serde_json::Value {
    match value {
        serde_json::Value::String(s) => {
            let mut out = "[REDACTED]".to_string();
            if !patterns.iter().any(|pattern| s.contains(pattern)) {
                out = s;
            }
            serde_json::Value::String(out)
        }
        serde_json::Value::Array(items) => serde_json::Value::Array(
            items
                .into_iter()
                .map(|item| redact(item, patterns))
                .collect(),
        ),
        serde_json::Value::Object(map) => serde_json::Value::Object(
            map.into_iter()
                .map(|(key, item)| (key, redact(item, patterns)))
                .collect(),
        ),
        other => other,
    }
}

// @test:IN-01-CONTRACT
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn trace_event_serde_round_trip() {
        let event = TraceEvent {
            schema_version: 1,
            trace_id: "trace-1".to_string(),
            parent_id: Some("parent-1".to_string()),
            task_id: "task-1".to_string(),
            seq: 3,
            ts_wall: "2026-11-24T00:00:00Z".to_string(),
            event: "run".to_string(),
            duration_ms: Some(42),
            cost: None,
            missing: vec!["x".to_string()],
            detail: None,
        };
        let json = serde_json::to_string(&event).unwrap();
        let back: TraceEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back.schema_version, event.schema_version);
        assert_eq!(back.trace_id, event.trace_id);
        assert_eq!(back.parent_id, event.parent_id);
        assert_eq!(back.task_id, event.task_id);
        assert_eq!(back.seq, event.seq);
        assert_eq!(back.ts_wall, event.ts_wall);
        assert_eq!(back.event, event.event);
        assert_eq!(back.duration_ms, event.duration_ms);
        assert!(back.cost.is_none());
        assert_eq!(back.missing, event.missing);
    }

    #[test]
    fn trace_span_requires_caller_supplied_ids() {
        let span = TraceSpan::new(
            "t".to_string(),
            Some("p".to_string()),
            "task".to_string(),
            "ev".to_string(),
            "wall".to_string(),
        );
        assert_eq!(span.trace_id, "t");
        assert_eq!(span.parent_id.as_deref(), Some("p"));
        assert_eq!(span.task_id, "task");
        assert_eq!(span.event, "ev");
        assert_eq!(span.ts_wall, "wall");
    }

    #[test]
    fn span_guard_finish_uses_supplied_seq_and_instant_duration() {
        let span = TraceSpan::new(
            "t".to_string(),
            None,
            "task".to_string(),
            "ev".to_string(),
            "2026-01-01T00:00:00Z".to_string(),
        );
        let guard = SpanGuard::new(span);
        let event = guard.finish(7);
        assert_eq!(event.seq, 7);
        assert_eq!(event.schema_version, 1);
        assert_eq!(event.ts_wall, "2026-01-01T00:00:00Z");
        assert!(event.duration_ms.is_some());
        assert!(event.duration_ms.unwrap() < 10_000);
    }

    // @test:IN-01-COST
    #[test]
    fn cost_source_serde_names_are_lowercase() {
        assert_eq!(
            serde_json::to_string(&CostSource::Measured).unwrap(),
            "\"measured\""
        );
        assert_eq!(
            serde_json::to_string(&CostSource::Estimated).unwrap(),
            "\"estimated\""
        );
        assert_eq!(
            serde_json::to_string(&CostSource::ProviderReported).unwrap(),
            "\"provider_reported\""
        );
        assert_eq!(
            serde_json::to_string(&CostSource::Unavailable).unwrap(),
            "\"unavailable\""
        );
    }

    #[test]
    fn cost_metadata_none_no_zero_substitution() {
        let cost = CostMetadata {
            value: None,
            currency: None,
            source: CostSource::Unavailable,
            missing: vec!["fee".to_string()],
        };
        let json = serde_json::to_string(&cost).unwrap();
        let back: CostMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(back.value, None);
        assert_eq!(back.source, CostSource::Unavailable);
        assert_eq!(back.missing, vec!["fee".to_string()]);
    }

    fn test_event(trace_id: &str, seq: u64, event: &str) -> TraceEvent {
        TraceEvent {
            schema_version: 1,
            trace_id: trace_id.to_string(),
            parent_id: None,
            task_id: "task-1".to_string(),
            seq,
            ts_wall: "2026-11-24T00:00:00Z".to_string(),
            event: event.to_string(),
            duration_ms: Some(5),
            cost: None,
            missing: Vec::new(),
            detail: None,
        }
    }

    fn rotated_segment_count(root: &Path, trace_id: &str) -> usize {
        let active = format!("{trace_id}.ndjson");
        let prefix = format!("{trace_id}.");
        fs::read_dir(root)
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                let name = entry.file_name().to_string_lossy().to_string();
                name != active && name.starts_with(&prefix) && name.ends_with(".ndjson")
            })
            .count()
    }

    // @test:IN-01-COLLECT
    #[test]
    fn off_telemetry_creates_no_directory_and_no_file() {
        let root = tempdir().unwrap();
        let config = TelemetryConfig {
            enabled: false,
            ..TelemetryConfig::default()
        };
        let mut collector = TraceCollector::new(root.path(), &config);
        assert!(!collector.enabled());
        collector
            .record(test_event("trace-off", 0, "start"))
            .unwrap();
        collector.flush("trace-off").unwrap();
        assert!(!collector.target_dir.exists());
    }

    #[test]
    fn close_returns_ok_on_empty_collector() {
        let root = tempdir().unwrap();
        let config = TelemetryConfig {
            enabled: true,
            ..TelemetryConfig::default()
        };
        let mut collector = TraceCollector::new(root.path(), &config);
        assert!(collector.close().is_ok());
        assert!(collector.buffer.is_empty());
    }

    #[test]
    fn enabled_telemetry_writes_one_ndjson_line_per_event_with_allowlist_filter() {
        let root = tempdir().unwrap();
        let config = TelemetryConfig {
            enabled: true,
            allowlist: vec!["trace_id".to_string(), "event".to_string()],
            ..TelemetryConfig::default()
        };
        let mut collector = TraceCollector::new(root.path(), &config);
        collector
            .record(test_event("trace-full", 0, "start"))
            .unwrap();
        collector
            .record(test_event("trace-full", 1, "next"))
            .unwrap();
        collector.flush("trace-full").unwrap();
        let path = collector.target_dir.join("trace-full.ndjson");
        let content = fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2);
        for line in lines {
            let value: serde_json::Value = serde_json::from_str(line).unwrap();
            let object = value.as_object().unwrap();
            assert_eq!(object.len(), 2);
            assert!(object.contains_key("trace_id"));
            assert!(object.contains_key("event"));
            assert!(!object.contains_key("seq"));
        }
        let metadata = fs::metadata(&path).unwrap();
        assert_eq!(PermissionsExt::mode(&metadata.permissions()) & 0o777, 0o600);
    }

    // @test:IN-01-PRIVACY
    #[test]
    fn redact_patterns_replace_matching_strings_including_nested() {
        let root = tempdir().unwrap();
        let config = TelemetryConfig {
            enabled: true,
            allowlist: vec![
                "trace_id".to_string(),
                "task_id".to_string(),
                "event".to_string(),
                "cost".to_string(),
                "missing".to_string(),
            ],
            redact_patterns: vec!["secret".to_string()],
            ..TelemetryConfig::default()
        };
        let mut collector = TraceCollector::new(root.path(), &config);
        let mut event = test_event("trace-redact", 0, "run");
        event.task_id = "secret-task-9".to_string();
        event.missing = vec!["safe".to_string(), "leak-secret-here".to_string()];
        event.cost = Some(CostMetadata {
            value: Some(1.5),
            currency: Some("secret-usd".to_string()),
            source: CostSource::Measured,
            missing: Vec::new(),
        });
        collector.record(event).unwrap();
        collector.flush("trace-redact").unwrap();
        let path = collector.target_dir.join("trace-redact.ndjson");
        let content = fs::read_to_string(&path).unwrap();
        let value: serde_json::Value = serde_json::from_str(content.trim()).unwrap();
        assert_eq!(
            value.get("task_id").and_then(|v| v.as_str()),
            Some("[REDACTED]")
        );
        assert_eq!(value.get("event").and_then(|v| v.as_str()), Some("run"));
        let missing = value.get("missing").and_then(|v| v.as_array()).unwrap();
        assert_eq!(missing.len(), 2);
        assert_eq!(missing[0], "safe");
        assert_eq!(missing[1], "[REDACTED]");
        let cost = value.get("cost").and_then(|v| v.as_object()).unwrap();
        assert_eq!(
            cost.get("currency").and_then(|v| v.as_str()),
            Some("[REDACTED]")
        );
    }

    // @test:IN-01-RETENTION
    #[test]
    fn purge_removes_active_and_rotated_segments_for_one_trace_only() {
        let root = tempdir().unwrap();
        let config = TelemetryConfig {
            enabled: true,
            ..TelemetryConfig::default()
        };
        let mut collector = TraceCollector::new(root.path(), &config);
        collector.record(test_event("trace-a", 0, "start")).unwrap();
        collector.flush("trace-a").unwrap();
        fs::write(
            collector.target_dir.join("trace-a.1000000000.ndjson"),
            "rotated\n",
        )
        .unwrap();
        collector.record(test_event("trace-b", 0, "begin")).unwrap();
        collector.flush("trace-b").unwrap();
        collector.purge("trace-a").unwrap();
        assert!(!collector.target_dir.join("trace-a.ndjson").exists());
        assert!(!root.path().join("trace-a.1000000000.ndjson").exists());
        assert!(collector.target_dir.join("trace-b.ndjson").exists());
        collector.purge("trace-missing").unwrap();
    }

    #[test]
    fn rotation_occurs_at_max_bytes_and_ttl_expires_rotated_segments() {
        let root = tempdir().unwrap();
        let config = TelemetryConfig {
            enabled: true,
            max_bytes: 1,
            ttl_secs: 86_400,
            ..TelemetryConfig::default()
        };
        let mut collector = TraceCollector::new(root.path(), &config);
        collector.record(test_event("trace-r", 0, "first")).unwrap();
        collector.flush("trace-r").unwrap();
        collector
            .record(test_event("trace-r", 1, "second"))
            .unwrap();
        collector.flush("trace-r").unwrap();
        assert_eq!(rotated_segment_count(&collector.target_dir, "trace-r"), 1);
        assert!(collector.target_dir.join("trace-r.ndjson").exists());
        collector.ttl_secs = 0;
        collector.record(test_event("trace-r", 2, "third")).unwrap();
        collector.flush("trace-r").unwrap();
        assert_eq!(rotated_segment_count(&collector.target_dir, "trace-r"), 0);
        assert!(collector.target_dir.join("trace-r.ndjson").exists());
    }

    // @test:IN-01-METRIC
    #[test]
    fn trace_report_has_no_aggregate_or_overall_score_field() {
        let report = TraceReport::from_events(&[], None, "commit-1", "on");
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
        assert!(!object.contains_key("score"));
        assert!(!object.contains_key("overall_score"));
        assert!(!object.contains_key("aggregate_score"));
        assert_eq!(report.event_coverage, "on");
    }

    #[test]
    fn zero_events_makes_every_metric_value_none() {
        let report = TraceReport::from_events(&[], None, "commit-1", "on");
        assert_eq!(report.baseline_commit, "commit-1");
        assert_eq!(report.skipped, Vec::<String>::new());
        assert_eq!(report.failures, Vec::<String>::new());
        assert_eq!(report.metrics.len(), 9);
        for metric in &report.metrics {
            assert_eq!(metric.value, None);
            assert_eq!(metric.denominator, Some(0));
        }
    }

    #[test]
    fn events_give_every_metric_a_denominator_and_at_least_one_value() {
        let events = vec![
            test_event("trace", 0, "gate_pass"),
            test_event("trace", 1, "gate_fail"),
            test_event("trace", 2, "defect"),
            test_event("trace", 3, "repro"),
            test_event("trace", 4, "tool call"),
            test_event("trace", 5, "token"),
            test_event("trace", 6, "recover"),
            event_with_cost("trace", 7, Some(3.0)),
            event_nested("trace", 8),
        ];
        let report = TraceReport::from_events(&events, None, "commit-1", "full");
        assert_eq!(report.metrics.len(), 9);
        let expected: Vec<&str> = vec![
            "gate_pass_rate",
            "defect_severity",
            "reproducibility",
            "tool_call_count",
            "token_count",
            "latency_ms",
            "cost_per_verified_result",
            "recovery_fidelity",
            "overhead",
        ];
        let names: Vec<&str> = report
            .metrics
            .iter()
            .map(|metric| metric.name.as_str())
            .collect();
        assert_eq!(names, expected);
        let gate = report
            .metrics
            .iter()
            .find(|metric| metric.name == "gate_pass_rate")
            .unwrap();
        assert_eq!(gate.value, Some(0.5));
        assert_eq!(gate.denominator, Some(2));
        let others: Vec<&Metric> = report
            .metrics
            .iter()
            .filter(|metric| metric.name != "cost_per_verified_result" && metric.name != "gate_pass_rate")
            .collect();
        for metric in others {
            assert_eq!(metric.denominator, Some(9));
        }
        let cost_metric = report
            .metrics
            .iter()
            .find(|metric| metric.name == "cost_per_verified_result")
            .unwrap();
        assert_eq!(cost_metric.value, Some(3.0));
        assert_eq!(cost_metric.denominator, Some(1));
        assert!(report.metrics.iter().any(|metric| metric.value.is_some()));
    }

    #[test]
    fn cost_per_verified_result_is_none_when_no_event_carries_cost() {
        let events = vec![test_event("trace", 0, "run"), test_event("trace", 1, "run")];
        let report = TraceReport::from_events(&events, None, "commit-1", "full");
        let metric = report
            .metrics
            .iter()
            .find(|metric| metric.name == "cost_per_verified_result")
            .unwrap();
        assert_eq!(metric.value, None);
        assert_eq!(metric.denominator, Some(0));
    }

    #[test]
    fn cost_per_verified_result_averages_only_cost_carriers() {
        let events = vec![
            event_with_cost("trace", 0, Some(2.0)),
            test_event("trace", 1, "run"),
            event_with_cost("trace", 2, Some(4.0)),
        ];
        let report = TraceReport::from_events(&events, None, "commit-1", "full");
        let metric = report
            .metrics
            .iter()
            .find(|metric| metric.name == "cost_per_verified_result")
            .unwrap();
        assert_eq!(metric.value, Some(3.0));
        assert_eq!(metric.denominator, Some(2));
    }

    #[test]
    fn skipped_and_failures_are_preserved_from_verification() {
        let events = vec![
            observation_event("trace", 0, true),
            observation_event("trace", 1, false),
            test_event("trace", 2, "task fail"),
            test_event("trace", 3, "error happened"),
        ];
        let verification = serde_json::json!({
            "status": "partial",
            "skipped": ["slow", "network"],
        });
        let report = TraceReport::from_events(&events, Some(&verification), "commit-1", "full");
        assert_eq!(
            report.skipped,
            vec!["slow".to_string(), "network".to_string()]
        );
        assert_eq!(
            report.failures,
            vec!["observation failed: task=task-1".to_string()]
        );
    }

    // @test:FR-AUD-18
    #[test]
    fn fr_aud_18_contract_examples() {
        // 1 pass + 1 fail + 8 unrelated events => gate pass rate 0.5
        let mut events: Vec<TraceEvent> = vec![
            bare_event("trace", 0, "gate_pass"),
            bare_event("trace", 1, "gate_fail"),
        ];
        for i in 2..10 {
            events.push(bare_event("trace", i, "run"));
        }
        // one event carries tokens: 500
        events.push(event_with_tokens("trace", 10, 500.0));
        // cost.value None and no duration_ms anywhere => both None
        let mut cost_event = event_with_cost("trace", 11, None);
        cost_event.duration_ms = None;
        events.push(cost_event);
        // failed observation => descriptive failure
        events.push(observation_event("trace", 12, false));

        let report = TraceReport::from_events(&events, None, "commit-1", "full");

        assert_eq!(metric_value(&report, "gate_pass_rate"), Some(0.5));
        assert_eq!(
            report
                .metrics
                .iter()
                .find(|metric| metric.name == "gate_pass_rate")
                .unwrap()
                .denominator,
            Some(2)
        );
        assert_eq!(metric_value(&report, "token_count"), Some(500.0));
        assert_eq!(metric_value(&report, "cost_per_verified_result"), None);
        assert_eq!(metric_value(&report, "latency_ms"), None);
        assert_eq!(
            report.failures,
            vec!["observation failed: task=task-1".to_string()]
        );
    }

    fn event_with_cost(trace_id: &str, seq: u64, value: Option<f64>) -> TraceEvent {
        let mut event = test_event(trace_id, seq, "run");
        event.cost = Some(CostMetadata {
            value,
            currency: Some("usd".to_string()),
            source: CostSource::Measured,
            missing: Vec::new(),
        });
        event
    }

    fn event_nested(trace_id: &str, seq: u64) -> TraceEvent {
        let mut event = test_event(trace_id, seq, "nested");
        event.parent_id = Some("parent-1".to_string());
        event
    }

    fn bare_event(trace_id: &str, seq: u64, event: &str) -> TraceEvent {
        TraceEvent {
            schema_version: 1,
            trace_id: trace_id.to_string(),
            parent_id: None,
            task_id: "task-1".to_string(),
            seq,
            ts_wall: "2026-11-24T00:00:00Z".to_string(),
            event: event.to_string(),
            duration_ms: None,
            cost: None,
            missing: Vec::new(),
            detail: None,
        }
    }

    fn observation_event(trace_id: &str, seq: u64, ok: bool) -> TraceEvent {
        let mut event = bare_event(trace_id, seq, "observation");
        event.detail = Some(serde_json::json!({ "ok": ok }));
        event
    }

    fn event_with_tokens(trace_id: &str, seq: u64, tokens: f64) -> TraceEvent {
        let mut event = bare_event(trace_id, seq, "tokens_seen");
        event.detail = Some(serde_json::json!({ "tokens": tokens }));
        event
    }

    fn metric_value(report: &TraceReport, name: &str) -> Option<f64> {
        report
            .metrics
            .iter()
            .find(|metric| metric.name == name)
            .expect("metric present")
            .value
    }

    // @test:IN-01-FILE-EVIDENCE
    #[test]
    fn file_change_tracker_unchanged_emits_zero_events() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("a.txt");
        fs::write(&file, "hello").unwrap();
        let mut tracker = FileChangeTracker::new(dir.path(), vec![file.clone()]);
        tracker.baseline().unwrap();
        let config = TelemetryConfig {
            enabled: true,
            ..TelemetryConfig::default()
        };
        let mut collector = TraceCollector::new(dir.path(), &config);
        let count = tracker
            .checkpoint(&mut collector, "task-1", "span-1")
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn file_change_tracker_changed_emits_exactly_one_event() {
        let dir = tempdir().unwrap();
        let rel = PathBuf::from("a.txt");
        let file = dir.path().join(&rel);
        fs::write(&file, "hello").unwrap();
        let before = hash_file(&file).unwrap();
        let mut tracker = FileChangeTracker::new(dir.path(), vec![rel.clone()]);
        tracker.baseline().unwrap();
        fs::write(&file, "world").unwrap();
        let after = hash_file(&file).unwrap();
        let config = TelemetryConfig {
            enabled: true,
            ..TelemetryConfig::default()
        };
        let mut collector = TraceCollector::new(dir.path(), &config);
        let count = tracker
            .checkpoint(&mut collector, "task-1", "span-1")
            .unwrap();
        assert_eq!(count, 1);
        let events = collector.buffer.get("task-1").unwrap();
        assert_eq!(events.len(), 1);
        let event = &events[0];
        assert_eq!(event.event, "file_change");
        assert_eq!(event.task_id, "task-1");
        assert_eq!(event.seq, 0);
        assert_eq!(event.duration_ms, None);
        assert!(event.cost.is_none());
        assert!(event.missing.is_empty());
        let detail = event.detail.as_ref().unwrap();
        assert_eq!(
            detail.get("path").and_then(|value| value.as_str()),
            Some("a.txt")
        );
        assert_eq!(
            detail.get("before").and_then(|value| value.as_str()),
            Some(before.as_str())
        );
        assert_eq!(
            detail.get("after").and_then(|value| value.as_str()),
            Some(after.as_str())
        );
        assert_eq!(
            detail.get("span").and_then(|value| value.as_str()),
            Some("span-1")
        );
    }

    // @test:IN-01-INTENT
    #[test]
    fn intent_event_carries_pre_write_hash() {
        let dir = tempdir().unwrap();
        let rel = PathBuf::from("b.txt");
        let file = dir.path().join(&rel);
        fs::write(&file, "hello").unwrap();
        let before = hash_file(&file).unwrap();
        let mut tracker = FileChangeTracker::new(dir.path(), vec![rel.clone()]);
        tracker.baseline().unwrap();
        let config = TelemetryConfig {
            enabled: true,
            ..TelemetryConfig::default()
        };
        let mut collector = TraceCollector::new(dir.path(), &config);
        tracker.intent(&mut collector, &file, "task-1").unwrap();
        fs::write(&file, "world").unwrap();
        let events = collector.buffer.get("task-1").unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event, "write_intent");
        let detail = events[0].detail.as_ref().unwrap();
        assert_eq!(
            detail.get("before").and_then(|value| value.as_str()),
            Some(before.as_str())
        );
        assert_eq!(
            detail.get("kind").and_then(|value| value.as_str()),
            Some("write_intent")
        );
    }

    #[test]
    fn file_change_and_intent_events_bypass_gating() {
        let dir = tempdir().unwrap();
        let rel = PathBuf::from("c.txt");
        let file = dir.path().join(&rel);
        fs::write(&file, "hello").unwrap();
        let mut tracker = FileChangeTracker::new(dir.path(), vec![rel.clone()]);
        tracker.baseline().unwrap();
        let config = TelemetryConfig {
            enabled: false,
            ..TelemetryConfig::default()
        };
        let mut collector = TraceCollector::new(dir.path(), &config);
        fs::write(&file, "world").unwrap();
        tracker
            .checkpoint(&mut collector, "task-1", "span-1")
            .unwrap();
        fs::write(&file, "hello").unwrap();
        tracker
            .checkpoint(&mut collector, "task-1", "span-1")
            .unwrap();
        tracker.intent(&mut collector, &file, "task-1").unwrap();
        let events = collector.buffer.get("task-1").unwrap();
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].event, "file_change");
        assert_eq!(events[1].event, "file_change");
        assert_eq!(events[2].event, "write_intent");
    }

    #[test]
    fn trace_event_without_detail_deserializes_to_none() {
        let json = r#"{"schema_version":1,"trace_id":"t","parent_id":null,"task_id":"task","seq":0,"ts_wall":"2026-01-01T00:00:00Z","event":"run","duration_ms":null,"cost":null,"missing":[]}"#;
        let event: TraceEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.detail, None);
    }
}
