//! Agent communication protocol core.
//!
//! File-based message store: envelopes live under `<run-dir>/messages/<id>.json`,
//! bodies under `<run-dir>/messages/<id>.body.json`. Writes use a temp file +
//! `rename` for atomicity. `validate_dir` audits every envelope in a run dir.
// @impl:FR-CAP-05

use std::collections::HashSet;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

const MESSAGES_DIR: &str = "messages";
const ENVELOPE_VERSION: u8 = 2;
const STATUS_SENT: &str = "sent";

/// Canonical protocol schema, loaded once from `policy/agent-protocol.toml`.
#[derive(Debug, Deserialize)]
pub struct ProtocolSchema {
    pub version: u64,
    pub statuses: Vec<String>,
    pub roles: Vec<RoleSchema>,
    pub kinds: Vec<KindSchema>,
}

#[derive(Debug, Deserialize)]
pub struct RoleSchema {
    pub name: String,
    pub kinds: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct KindSchema {
    pub name: String,
    #[serde(default)]
    pub required: Vec<String>,
    #[serde(default)]
    pub array_fields: Vec<String>,
    #[serde(default)]
    pub optional_numeric: Vec<String>,
    #[serde(default)]
    pub optional_array: Vec<String>,
    #[serde(default)]
    pub nested: Vec<NestedField>,
    #[serde(default)]
    pub requires_schema_id: bool,
}

#[derive(Debug, Deserialize)]
pub struct NestedField {
    pub field: String,
    pub required: Vec<String>,
    #[serde(default)]
    pub numeric: bool,
}

static PROTOCOL_SCHEMA: OnceLock<ProtocolSchema> = OnceLock::new();

/// Loads the canonical protocol schema once from `policy/agent-protocol.toml`.
pub fn protocol_schema() -> &'static ProtocolSchema {
    PROTOCOL_SCHEMA.get_or_init(|| {
        let data = include_str!("../../policy/agent-protocol.toml");
        toml::from_str(data).expect("failed to parse policy/agent-protocol.toml")
    })
}

// @impl:FR-LEADER-02
/// Every role allowed on the bus.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    #[serde(rename = "leader")]
    Leader,
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
            Role::Leader => "leader",
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

/// The common envelope. Files write full semantic keys (v, id, from, to, kind,
/// reply_to, ref, status, ts). `ref` is serialized as `"ref"` because `ref` is
/// a Rust keyword; that is a semantic name, not a shortening. No `body` field:
/// bodies travel in sidecar files so tool calls carry only id/path/hash.
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
        let entries = match fs::read_dir(messages_dir) {
            Ok(e) => e,
            Err(_) => {
                errors.push(format!(
                    "messages directory missing or unreadable: {}",
                    messages_dir.display()
                ));
                return Err(errors);
            }
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
        if !errors.is_empty() {
            return Err(errors);
        }
        Ok(())
    }

    pub fn send(
        &self,
        from: Role,
        to: Role,
        kind: Kind,
        body: Value,
        ref_path: Option<&Path>,
        reply_to: Option<&str>,
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
            reply_to: reply_to.map(|s| s.to_string()),
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
        let entries = match fs::read_dir(&self.messages_dir) {
            Ok(e) => e,
            Err(_) => return msgs,
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
            let envelope = match read_envelope_file(&path) {
                Ok(e) => e,
                Err(_) => continue,
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
        if !protocol_schema().statuses.iter().any(|s| s == status) || status == STATUS_SENT {
            return Err(format!("invalid ack status: {status}"));
        }
        let env_path = self.messages_dir.join(format!("{id}.json"));
        let mut envelope = read_envelope_file(&env_path)?;
        let is_recipient = envelope.to == agent;
        let is_leader_rejecting_own =
            agent == Role::Leader && envelope.from == Role::Leader && status == "rejected";
        if !is_recipient && !is_leader_rejecting_own {
            return Err(format!(
                "only recipient {} may ack {}",
                envelope.to.as_str(),
                id
            ));
        }
        envelope.status = status.to_string();
        write_json_atomic(&env_path, &envelope)?;
        Ok(())
    }

    fn kind_allowed(from: &Role, kind: &Kind) -> bool {
        // CUSTOM is allowed for every role.
        if matches!(kind, Kind::Custom) {
            return true;
        }
        let schema = protocol_schema();
        let role = match schema.roles.iter().find(|r| r.name == from.as_str()) {
            Some(r) => r,
            None => return false,
        };
        role.kinds.iter().any(|k| k == kind_as_str(kind))
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
        let mut suffix = random_id_suffix();
        loop {
            let id = format!("msg-{ms}-{suffix}");
            if !self.messages_dir.join(format!("{id}.json")).exists() {
                return id;
            }
            suffix = random_id_suffix();
        }
    }
}

/// 8 bytes from /dev/urandom encoded as hex, falling back to epoch nanos
/// when the device is unavailable.
fn random_id_suffix() -> String {
    let mut buf = [0u8; 8];
    if std::fs::File::open("/dev/urandom")
        .and_then(|mut file| file.read_exact(&mut buf))
        .is_ok()
    {
        return buf.iter().map(|byte| format!("{byte:02x}")).collect();
    }
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .subsec_nanos();
    format!("{nanos:06x}")
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

fn require_keys(obj: &Map<String, Value>, keys: &[String]) -> Result<(), String> {
    for key in keys {
        if !obj.contains_key(key) {
            return Err(format!("body missing required key: {key}"));
        }
    }
    Ok(())
}

fn validate_body(kind: &Kind, body: &Value) -> Result<(), String> {
    let schema = protocol_schema();
    let obj = body
        .as_object()
        .ok_or_else(|| "body must be a JSON object".to_string())?;
    let kind_schema = match schema.kinds.iter().find(|k| k.name == kind_as_str(kind)) {
        Some(k) => k,
        None => return Err(format!("unknown kind: {}", kind_as_str(kind))),
    };
    require_keys(obj, &kind_schema.required)?;
    for field in &kind_schema.array_fields {
        obj.get(field)
            .and_then(Value::as_array)
            .ok_or_else(|| format!("{field} must be an array"))?;
    }
    for field in &kind_schema.optional_numeric {
        let value = match obj.get(field) {
            Some(value) => value,
            None => continue,
        };
        if !matches!(value, Value::Number(_)) {
            return Err(format!("{field} must be numeric"));
        }
    }
    for field in &kind_schema.optional_array {
        let value = match obj.get(field) {
            Some(value) => value,
            None => continue,
        };
        if !matches!(value, Value::Array(_)) {
            return Err(format!("{field} must be an array"));
        }
    }
    for nested in &kind_schema.nested {
        let parent = obj
            .get(&nested.field)
            .and_then(Value::as_object)
            .ok_or_else(|| format!("{} must be an object", nested.field))?;
        require_keys(parent, &nested.required)?;
        if nested.numeric {
            validate_nested_numeric(parent, nested)?;
        }
    }
    if kind_schema.requires_schema_id {
        obj.get("schema_id")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| "CUSTOM requires a non-empty schema_id".to_string())?;
    }
    Ok(())
}

/// Validates that the required numeric fields of a nested object are numbers.
fn validate_nested_numeric(
    parent: &Map<String, Value>,
    nested: &NestedField,
) -> Result<(), String> {
    for key in &nested.required {
        let value = parent
            .get(key)
            .ok_or_else(|| format!("{}.{} missing", nested.field, key))?;
        if !matches!(value, Value::Number(_)) {
            return Err(format!("{}.{} must be numeric", nested.field, key));
        }
    }
    Ok(())
}

fn validate_envelope(
    dir: &Path,
    envelope: &Envelope,
    ids: &HashSet<String>,
    errors: &mut Vec<String>,
) {
    let prefix = format!("envelope {}", envelope.id);
    if !protocol_schema()
        .statuses
        .iter()
        .any(|s| s == &envelope.status)
    {
        errors.push(format!("{prefix}: invalid status {}", envelope.status));
    }
    if !MessageStore::kind_allowed(&envelope.from, &envelope.kind) {
        errors.push(format!(
            "{prefix}: role {} may not send kind {}",
            envelope.from.as_str(),
            kind_as_str(&envelope.kind)
        ));
    }
    match &envelope.reply_to {
        Some(reply_to) if !ids.contains(reply_to) => {
            errors.push(format!("{prefix}: orphan reply_to {reply_to}"));
        }
        _ => {}
    }
    if let Some(ref_) = &envelope.ref_ {
        match validate_ref(dir, ref_) {
            Ok(()) => {}
            Err(e) => errors.push(format!("{prefix}: ref {e}")),
        }
    }
    let body_path = dir
        .join(MESSAGES_DIR)
        .join(format!("{}.body.json", envelope.id));
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
    if hash.len() != 64
        || !hash
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, 'a'..='f'))
    {
        return Err(format!("hash must be 64 lowercase hex chars: {hash}"));
    }
    let full = dir.join(rel);
    let full_canon = match full.canonicalize() {
        Ok(c) => c,
        Err(_) => return Err(format!("ref file not found: {rel}")),
    };
    let dir_canon = match dir.canonicalize() {
        Ok(c) => c,
        Err(_) => return Err(format!("run dir not accessible: {}", dir.display())),
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

pub fn hash_file(path: &Path) -> Result<String, String> {
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
    let envelope: Envelope = serde_json::from_str(&data).map_err(|e| e.to_string())?;
    if envelope.v != ENVELOPE_VERSION {
        return Err(format!("unknown envelope version {}", envelope.v));
    }
    Ok(envelope)
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
    let era = match z >= 0 {
        true => z,
        false => z - 146_096,
    } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = match mp < 10 {
        true => mp + 3,
        false => mp - 9,
    };
    let y = match m <= 2 {
        true => y + 1,
        false => y,
    };
    (y, m as u32, d as u32)
}

pub fn now_rfc3339() -> String {
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
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}
