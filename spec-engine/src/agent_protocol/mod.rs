//! Agent communication protocol core.
//!
//! File-based message store: envelopes live under `<run-dir>/messages/<id>.json`,
//! bodies under `<run-dir>/messages/<id>.body.json`. Writes use a temp file +
//! `rename` for atomicity. `validate_dir` audits every envelope in a run dir.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

const MESSAGES_DIR: &str = "messages";
const ENVELOPE_VERSION: u8 = 1;
const STATUS_SENT: &str = "sent";

const ALLOWED_STATUSES: &[&str] = &["sent", "accepted", "rejected", "done"];

/// Every role allowed on the bus.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    #[serde(rename = "lead")]
    Lead,
    #[serde(rename = "executor")]
    Executor,
    #[serde(rename = "judge")]
    Judge,
    #[serde(rename = "explorer")]
    Explorer,
}

impl Role {
    pub fn as_str(&self) -> &str {
        match self {
            Role::Lead => "lead",
            Role::Executor => "executor",
            Role::Judge => "judge",
            Role::Explorer => "explorer",
        }
    }
}

/// Message kind. `CUSTOM` requires a registered `schema_id` in the body.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    #[serde(rename = "TASK_DISPATCH")]
    TaskDispatch,
    #[serde(rename = "RULING")]
    Ruling,
    #[serde(rename = "TASK_REPORT")]
    TaskReport,
    #[serde(rename = "VERDICT")]
    Verdict,
    #[serde(rename = "MAP_REPORT")]
    MapReport,
    #[serde(rename = "CUSTOM")]
    Custom,
}

/// The common envelope. Fields serialize in protocol order: v, id, from, to,
/// kind, reply_to, ref, status, ts. No `body` field: bodies travel in sidecar
/// files so tool calls carry only id/path/hash.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Envelope {
    pub v: u8,
    pub id: String,
    pub from: Role,
    pub to: Role,
    pub kind: Kind,
    pub reply_to: Option<String>,
    #[serde(rename = "ref")]
    pub ref_: Option<String>,
    pub status: String,
    pub ts: String,
}

pub struct MessageStore {
    run_dir: PathBuf,
    messages_dir: PathBuf,
}

pub fn validate_dir(dir: &Path) -> Result<(), Vec<String>> {
    MessageStore::new(dir).validate_dir()
}

impl MessageStore {
    pub fn new(run_dir: &Path) -> Self {
        let messages_dir = run_dir.join(MESSAGES_DIR);
        Self {
            run_dir: run_dir.to_path_buf(),
            messages_dir,
        }
    }

    fn validate_dir(&self) -> Result<(), Vec<String>> {
        let messages_dir = &self.messages_dir;
        let mut errors: Vec<String> = Vec::new();
        let mut ids: HashSet<String> = HashSet::new();
        let mut envelopes: Vec<Envelope> = Vec::new();
        let Ok(entries) = fs::read_dir(messages_dir) else {
            errors.push(format!(
                "messages directory missing or unreadable: {}",
                messages_dir.display()
            ));
            return Err(errors);
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };
            if name.ends_with(".body.json") {
                continue;
            }
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            match read_envelope_file(&path) {
                Ok(envelope) => {
                    ids.insert(envelope.id.clone());
                    envelopes.push(envelope);
                }
                Err(e) => errors.push(format!("{name}: {e}")),
            }
        }
        for envelope in envelopes {
            validate_envelope(&self.run_dir, &envelope, &ids, &mut errors);
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    pub fn send(
        &self,
        from: Role,
        to: Role,
        kind: Kind,
        body: Value,
        ref_path: Option<&Path>,
    ) -> Result<Envelope, String> {
        if !Self::kind_allowed(&from, &kind) {
            return Err(format!(
                "role {} is not allowed to send kind {}",
                from.as_str(),
                kind_as_str(&kind)
            ));
        }
        validate_body(&kind, &body)?;
        let ref_ = match ref_path {
            None => None,
            Some(path) => {
                let rel = self.rel_path(path)?;
                let hash = hash_file(path)?;
                Some(format!("{}@sha256:{}", rel.display(), hash))
            }
        };
        let id = self.next_id();
        let ts = now_rfc3339();
        let envelope = Envelope {
            v: ENVELOPE_VERSION,
            id: id.clone(),
            from,
            to,
            kind,
            reply_to: None,
            ref_,
            status: STATUS_SENT.to_string(),
            ts,
        };
        let body_path = self.messages_dir.join(format!("{id}.body.json"));
        let env_path = self.messages_dir.join(format!("{id}.json"));
        fs::create_dir_all(&self.messages_dir).map_err(|e| e.to_string())?;
        write_json_atomic(&body_path, &body)?;
        write_json_atomic(&env_path, &envelope)?;
        Ok(envelope)
    }

    pub fn recv(&self, agent: Role) -> Vec<Envelope> {
        let mut msgs: Vec<Envelope> = Vec::new();
        let Ok(entries) = fs::read_dir(&self.messages_dir) else {
            return msgs;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };
            if name.ends_with(".body.json") {
                continue;
            }
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let Ok(envelope) = read_envelope_file(&path) else {
                continue;
            };
            if envelope.to == agent && envelope.status == STATUS_SENT {
                msgs.push(envelope);
            }
        }
        msgs.sort_by(|a, b| a.ts.cmp(&b.ts));
        msgs
    }

    pub fn read(&self, id: &str) -> Result<Value, String> {
        let env_path = self.messages_dir.join(format!("{id}.json"));
        if !env_path.exists() {
            return Err(format!("message {id} not found"));
        }
        read_envelope_file(&env_path)?; // surface malformed envelopes
        let body_path = self.messages_dir.join(format!("{id}.body.json"));
        let data = fs::read_to_string(&body_path)
            .map_err(|e| format!("cannot read body for {id}: {e}"))?;
        serde_json::from_str(&data).map_err(|e| e.to_string())
    }

    pub fn ack(&self, agent: Role, id: &str, status: &str) -> Result<(), String> {
        if !ALLOWED_STATUSES.contains(&status) || status == STATUS_SENT {
            return Err(format!("invalid ack status: {status}"));
        }
        let env_path = self.messages_dir.join(format!("{id}.json"));
        let mut envelope = read_envelope_file(&env_path)?;
        let is_recipient = envelope.to == agent;
        let is_lead_rejecting_own =
            agent == Role::Lead && envelope.from == Role::Lead && status == "rejected";
        if !is_recipient && !is_lead_rejecting_own {
            return Err(format!("only recipient {} may ack {}", envelope.to.as_str(), id));
        }
        envelope.status = status.to_string();
        write_json_atomic(&env_path, &envelope)?;
        Ok(())
    }

    fn kind_allowed(from: &Role, kind: &Kind) -> bool {
        matches!(
            (from, kind),
            (Role::Lead, Kind::TaskDispatch)
                | (Role::Lead, Kind::Ruling)
                | (Role::Executor, Kind::TaskReport)
                | (Role::Judge, Kind::Verdict)
                | (Role::Explorer, Kind::MapReport)
                | (_, Kind::Custom)
        )
    }

    fn rel_path(&self, path: &Path) -> Result<PathBuf, String> {
        path.strip_prefix(&self.run_dir)
        .map(|rel| rel.to_path_buf())
        .map_err(|_| format!("ref {path:?} escapes run dir {}", self.run_dir.display()))
    }

    fn next_id(&self) -> String {
        let epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before unix epoch");
        let ms = epoch.as_millis();
        let mut sub = epoch.subsec_nanos() % 1_000_000;
        loop {
            let id = format!("msg-{ms}-{sub:06x}");
            if !self
                .messages_dir
                .join(format!("{id}.json"))
                .exists()
            {
                return id;
            }
            sub = sub.wrapping_add(1);
        }
    }
}

fn kind_as_str(kind: &Kind) -> &str {
    match kind {
        Kind::TaskDispatch => "TASK_DISPATCH",
        Kind::Ruling => "RULING",
        Kind::TaskReport => "TASK_REPORT",
        Kind::Verdict => "VERDICT",
        Kind::MapReport => "MAP_REPORT",
        Kind::Custom => "CUSTOM",
    }
}

fn require_keys(obj: &Map<String, Value>, keys: &[&str]) -> Result<(), String> {
    for key in keys {
        if !obj.contains_key(*key) {
            return Err(format!("body missing required key: {key}"));
        }
    }
    Ok(())
}

fn validate_body(kind: &Kind, body: &Value) -> Result<(), String> {
    let obj = body
        .as_object()
        .ok_or_else(|| "body must be a JSON object".to_string())?;
    match kind {
        Kind::TaskDispatch => require_keys(
            obj,
            &["task", "files", "acceptance", "verification_command"],
        ),
        Kind::Ruling => require_keys(obj, &["finding", "decision", "cost_if_wrong"]),
        Kind::TaskReport => {
            require_keys(obj, &["status", "commits", "tests", "concerns"])?;
            obj.get("concerns")
                .and_then(Value::as_array)
                .ok_or_else(|| "concerns must be an array".to_string())?;
            let tests = obj
                .get("tests")
                .and_then(Value::as_object)
                .ok_or_else(|| "tests must be an object".to_string())?;
            require_keys(tests, &["exit_code", "ref"])?;
            Ok(())
        }
        Kind::Verdict => {
            require_keys(
                obj,
                &[
                    "verdict",
                    "blocking",
                    "non_blocking",
                    "missing_evidence",
                    "next_action",
                    "rerun",
                ],
            )?;
            let rerun = obj
                .get("rerun")
                .and_then(Value::as_object)
                .ok_or_else(|| "rerun must be an object".to_string())?;
            let numeric = |v: &Value| matches!(v, Value::Number(_));
            let test_v = rerun
                .get("test")
                .ok_or_else(|| "rerun.test missing".to_string())?;
            let check_v = rerun
                .get("check")
                .ok_or_else(|| "rerun.check missing".to_string())?;
            if !numeric(test_v) || !numeric(check_v) {
                return Err("rerun.test and rerun.check must be numeric".to_string());
            }
            Ok(())
        }
        Kind::MapReport => require_keys(obj, &["modules", "read_only"]),
        Kind::Custom => {
            obj.get("schema_id")
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| "CUSTOM requires a non-empty schema_id".to_string())?;
            Ok(())
        }
    }
}

fn validate_envelope(
    dir: &Path,
    envelope: &Envelope,
    ids: &HashSet<String>,
    errors: &mut Vec<String>,
) {
    let prefix = format!("envelope {}", envelope.id);
    if envelope.v != ENVELOPE_VERSION {
        errors.push(format!("{prefix}: unknown envelope version {}", envelope.v));
    }
    if !ALLOWED_STATUSES.contains(&envelope.status.as_str()) {
        errors.push(format!("{prefix}: invalid status {}", envelope.status));
    }
    if !MessageStore::kind_allowed(&envelope.from, &envelope.kind) {
        errors.push(format!(
            "{prefix}: role {} may not send kind {}",
            envelope.from.as_str(),
            kind_as_str(&envelope.kind)
        ));
    }
    if let Some(reply_to) = &envelope.reply_to {
        if !ids.contains(reply_to) {
            errors.push(format!("{prefix}: orphan reply_to {reply_to}"));
        }
    }
    if let Some(ref_) = &envelope.ref_ {
        match validate_ref(dir, ref_) {
            Ok(()) => {}
            Err(e) => errors.push(format!("{prefix}: ref {e}")),
        }
    }
    let body_path = dir.join(MESSAGES_DIR).join(format!("{}.body.json", envelope.id));
    let body = match fs::read_to_string(&body_path) {
        Ok(data) => match serde_json::from_str::<Value>(&data) {
            Ok(b) => b,
            Err(e) => {
                errors.push(format!("{prefix}: missing or invalid body: {e}"));
                return;
            }
        },
        Err(e) => {
            errors.push(format!("{prefix}: missing or invalid body: {e}"));
            return;
        }
    };
    if let Err(e) = validate_body(&envelope.kind, &body) {
        errors.push(format!("{prefix}: body {e}"));
    }
}

fn validate_ref(dir: &Path, ref_: &str) -> Result<(), String> {
    let mut parts = ref_.splitn(2, "@sha256:");
    let rel = parts.next().ok_or("empty ref path")?;
    let hash = parts.next().ok_or("ref missing @sha256:<hash>")?;
    if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_digit() || matches!(c, 'a'..='f')) {
        return Err(format!("hash must be 64 lowercase hex chars: {hash}"));
    }
    let full = dir.join(rel);
    let Ok(full_canon) = full.canonicalize() else {
        return Err(format!("ref file not found: {rel}"));
    };
    let Ok(dir_canon) = dir.canonicalize() else {
        return Err(format!("run dir not accessible: {}", dir.display()));
    };
    if !full_canon.starts_with(&dir_canon) {
        return Err(format!("cross-run ref: {rel}"));
    }
    let actual = hash_file(&full_canon)?;
    if actual != hash {
        return Err(format!("hash mismatch for {rel}"));
    }
    Ok(())
}

fn hash_file(path: &Path) -> Result<String, String> {
    let data = fs::read(path).map_err(|e| format!("cannot read {path:?}: {e}"))?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    Ok(hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

fn read_envelope_file(path: &Path) -> Result<Envelope, String> {
    let data = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&data).map_err(|e| e.to_string())
}

fn write_json_atomic(path: &Path, value: &impl Serialize) -> Result<(), String> {
    let json = serde_json::to_string(value).map_err(|e| e.to_string())?;
    let dir = path
        .parent()
        .ok_or_else(|| format!("no parent dir for {:?}", path))?;
    let tmp = dir.join(format!(
        ".{}.tmp",
        path.file_name().unwrap().to_string_lossy()
    ));
    fs::write(&tmp, json).map_err(|e| e.to_string())?;
    fs::rename(&tmp, path).map_err(|e| e.to_string())?;
    Ok(())
}

// Days since 1970-01-01 -> (year, month, day), civil calendar algorithm.
fn civil_from_days(mut z: i64) -> (i64, u32, u32) {
    z += 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m as u32, d as u32)
}

fn now_rfc3339() -> String {
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch");
    let secs = dur.as_secs();
    let days = secs / 86_400;
    let secs_of_day = secs % 86_400;
    let (year, month, day) = civil_from_days(days as i64);
    let hour = secs_of_day / 3_600;
    let minute = (secs_of_day % 3_600) / 60;
    let second = secs_of_day % 60;
    format!(
        "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z"
    )
}
