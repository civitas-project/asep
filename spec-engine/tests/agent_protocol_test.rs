// @test:FR-CAP-05
use asep::agent_protocol::{Kind, MessageStore, Role};
use serde_json::json;
use std::fs;

fn dispatch_body() -> serde_json::Value {
    json!({
        "task": "implement feature",
        "files": ["src/feature.rs"],
        "acceptance": "feature works",
        "verification_command": "cargo test -p asep"
    })
}

fn verdict_body() -> serde_json::Value {
    json!({
        "verdict": "PASS",
        "blocking": [],
        "non_blocking": [],
        "missing_evidence": [],
        "next_action": "proceed",
        "rerun": {"test": 1, "check": 0}
    })
}

fn task_report_body(concerns: serde_json::Value) -> serde_json::Value {
    json!({
        "status": "done",
        "commits": ["abc123"],
        "tests": {"exit_code": 0, "ref": "tests/x.test@sha256:deadbeef"},
        "concerns": concerns
    })
}

#[test]
fn test_task_report_requires_concerns() {
    let dir = tempfile::tempdir().unwrap();
    let store = MessageStore::new(dir.path());
    let mut no_concerns = task_report_body(json!([]));
    no_concerns.as_object_mut().unwrap().remove("concerns");
    assert!(store
        .send(Role::Executor, Role::Lead, Kind::TaskReport, no_concerns, None, None)
        .is_err());
    assert!(store
        .send(
            Role::Executor,
            Role::Lead,
            Kind::TaskReport,
            task_report_body(json!([])),
            None,
            None
        )
        .is_ok());
    assert!(store
        .send(
            Role::Executor,
            Role::Lead,
            Kind::TaskReport,
            task_report_body(json!(["draft spec"])),
            None,
            None
        )
        .is_ok());
}

#[test]
fn test_lead_can_reject_own_sent_message() {
    let dir = tempfile::tempdir().unwrap();
    let store = MessageStore::new(dir.path());
    let envelope = store
        .send(
            Role::Lead,
            Role::Executor,
            Kind::TaskDispatch,
            dispatch_body(),
            None,
            None,
        )
        .unwrap();
    // Lead may reject a message it sent.
    store.ack(Role::Lead, &envelope.id, "rejected").unwrap();
    let env: serde_json::Value = serde_json::from_str(&fs::read_to_string(
        dir.path().join("messages").join(format!("{}.json", envelope.id)),
    )
    .unwrap())
    .unwrap();
    assert_eq!(env["status"], "rejected");
    // Non-recipient cannot ack lead's message.
    assert!(store.ack(Role::Judge, &envelope.id, "accepted").is_err());
    // Lead may not ack its own message with a non-rejected status.
    let other = store
        .send(
            Role::Lead,
            Role::Executor,
            Kind::TaskDispatch,
            dispatch_body(),
            None,
            None,
        )
        .unwrap();
    assert!(store.ack(Role::Lead, &other.id, "accepted").is_err());
    assert!(store.ack(Role::Lead, &other.id, "done").is_err());
    // Lead cannot reject a message it did not send (and is not recipient of).
    let not_own = store
        .send(
            Role::Executor,
            Role::Judge,
            Kind::TaskReport,
            task_report_body(json!([])),
            None,
            None,
        )
        .unwrap();
    assert!(store.ack(Role::Lead, &not_own.id, "rejected").is_err());
}

#[test]
fn test_send_writes_envelope_without_body_in_summary() {
    let dir = tempfile::tempdir().unwrap();
    let store = MessageStore::new(dir.path());
    let envelope = store
        .send(
            Role::Lead,
            Role::Executor,
            Kind::TaskDispatch,
            dispatch_body(),
            None,
            None,
        )
        .unwrap();
    let summary = serde_json::to_string(&envelope).unwrap();
    assert!(summary.contains("\"id\""));
    assert!(summary.contains("\"kind\""));
    assert!(summary.contains("\"from\""));
    assert!(summary.contains("\"to\""));
    assert!(!summary.contains("body"));
    assert_eq!(envelope.from, Role::Lead);
    assert_eq!(envelope.to, Role::Executor);
    assert_eq!(envelope.kind, Kind::TaskDispatch);
    assert_eq!(envelope.status, "sent");
    assert!(envelope.id.starts_with("msg-"));
    let env_path = dir.path().join("messages").join(format!("{}.json", envelope.id));
    assert!(env_path.exists());
}

#[test]
fn test_ref_hash_mismatch_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let store = MessageStore::new(dir.path());
    let ref_path = dir.path().join("brief.md");
    fs::write(&ref_path, "original content").unwrap();
    store
        .send(
            Role::Lead,
            Role::Executor,
            Kind::TaskDispatch,
            dispatch_body(),
            Some(&ref_path),
            None,
        )
        .unwrap();
    fs::write(&ref_path, "tampered content").unwrap();
    let errors = asep::agent_protocol::validate_dir(dir.path()).unwrap_err();
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.contains("hash") || e.contains("mismatch") || e.contains("brief.md")));
}

#[test]
fn test_role_kind_mismatch_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let store = MessageStore::new(dir.path());
    let result = store.send(
        Role::Executor,
        Role::Lead,
        Kind::TaskDispatch,
        dispatch_body(),
        None,
        None,
    );
    assert!(result.is_err());
}

#[test]
fn test_recv_returns_only_unacked_for_recipient() {
    let dir = tempfile::tempdir().unwrap();
    let store = MessageStore::new(dir.path());
    let e1 = store
        .send(Role::Lead, Role::Executor, Kind::TaskDispatch, dispatch_body(), None, None)
        .unwrap();
    let e2 = store
        .send(Role::Lead, Role::Executor, Kind::TaskDispatch, dispatch_body(), None, None)
        .unwrap();
    let e3 = store
        .send(Role::Judge, Role::Executor, Kind::Verdict, verdict_body(), None, None)
        .unwrap();
    store.ack(Role::Executor, &e1.id, "accepted").unwrap();
    let inbox: Vec<_> = store.recv(Role::Executor);
    assert_eq!(inbox.len(), 2);
    assert!(inbox.iter().any(|e| e.id == e2.id));
    assert!(inbox.iter().any(|e| e.id == e3.id));
    assert!(inbox.iter().all(|e| e.status == "sent"));
    // Judge cannot ack executor's message (recipient-only enforcement).
    assert!(store.ack(Role::Judge, &e2.id, "accepted").is_err());
}

#[test]
fn test_orphan_reply_to_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let _store = MessageStore::new(dir.path());
    let orphan = serde_json::json!({
        "v": 1,
        "id": "msg-1-orphan",
        "from": "lead",
        "to": "executor",
        "kind": "TASK_DISPATCH",
        "reply_to": "msg-0-does-not-exist",
        "ref": null,
        "status": "sent",
        "ts": "2026-11-20T10:00:00Z",
    });
    fs::create_dir_all(dir.path().join("messages")).unwrap();
    fs::write(
        dir.path().join("messages").join("msg-1-orphan.json"),
        serde_json::to_string(&orphan).unwrap(),
    )
    .unwrap();
    fs::write(
        dir.path().join("messages").join("msg-1-orphan.body.json"),
        serde_json::to_string(&dispatch_body()).unwrap(),
    )
    .unwrap();
    let errors = asep::agent_protocol::validate_dir(dir.path()).unwrap_err();
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.contains("reply_to")));
}

#[test]
fn test_protocol_schema_loads() {
    let schema = asep::agent_protocol::protocol_schema();
    assert_eq!(schema.version, 1);
    assert_eq!(schema.roles.len(), 4);
    assert_eq!(
        schema.roles.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(),
        vec!["lead", "executor", "judge", "explorer"]
    );
    assert_eq!(schema.kinds.len(), 6);
    assert_eq!(
        schema.kinds.iter().map(|k| k.name.as_str()).collect::<Vec<_>>(),
        vec![
            "TASK_DISPATCH",
            "RULING",
            "TASK_REPORT",
            "VERDICT",
            "MAP_REPORT",
            "CUSTOM"
        ]
    );
}

#[test]
fn test_schema_drives_role_kind_check() {
    let dir = tempfile::tempdir().unwrap();
    let store = MessageStore::new(dir.path());
    // executor may not send TASK_DISPATCH per schema.
    assert!(store
        .send(Role::Executor, Role::Lead, Kind::TaskDispatch, dispatch_body(), None, None)
        .is_err());
    // executor may send TASK_REPORT per schema.
    assert!(store
        .send(
            Role::Executor,
            Role::Lead,
            Kind::TaskReport,
            task_report_body(json!([])),
            None,
            None,
        )
        .is_ok());
}

#[test]
fn test_schema_drives_body_check() {
    let dir = tempfile::tempdir().unwrap();
    let store = MessageStore::new(dir.path());
    // TASK_REPORT missing required "concerns" is rejected.
    let mut no_concerns = task_report_body(json!([]));
    no_concerns.as_object_mut().unwrap().remove("concerns");
    assert!(store
        .send(Role::Executor, Role::Lead, Kind::TaskReport, no_concerns, None, None)
        .is_err());
    // VERDICT rerun with non-numeric test is rejected.
    let mut bad_verdict = verdict_body();
    bad_verdict
        .as_object_mut()
        .unwrap()
        .get_mut("rerun")
        .unwrap()["test"] = json!("one");
    assert!(store
        .send(Role::Judge, Role::Lead, Kind::Verdict, bad_verdict, None, None)
        .is_err());
    // numeric rerun is accepted.
    assert!(store
        .send(Role::Judge, Role::Lead, Kind::Verdict, verdict_body(), None, None)
        .is_ok());
}

#[test]
fn test_envelope_uses_full_semantic_keys() {
    let dir = tempfile::tempdir().unwrap();
    let store = MessageStore::new(dir.path());
    let envelope = store
        .send(
            Role::Lead,
            Role::Executor,
            Kind::TaskDispatch,
            dispatch_body(),
            None,
            None,
        )
        .unwrap();
    let raw = fs::read_to_string(
        dir.path().join("messages").join(format!("{}.json", envelope.id)),
    )
    .unwrap();
    let obj: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let keys = obj.as_object().expect("envelope is a JSON object");
    for key in ["id", "from", "to", "kind", "reply_to", "ref", "status"] {
        assert!(keys.contains_key(key), "envelope must write '{key}' key");
    }
    for short in ["f", "t", "k", "rt", "r", "s"] {
        assert!(!keys.contains_key(short), "envelope must not contain short key '{short}'");
    }

    // Legacy long-key envelope must still deserialize and pass validation.
    // Long keys are now the only format, so this is the same format.
    let legacy = serde_json::json!({
        "v": 1,
        "id": "msg-legacy-001",
        "from": "lead",
        "to": "executor",
        "kind": "TASK_DISPATCH",
        "reply_to": null,
        "ref": null,
        "status": "sent",
        "ts": "2026-11-20T10:00:00Z",
    });
    fs::create_dir_all(dir.path().join("messages")).unwrap();
    fs::write(
        dir.path().join("messages").join("msg-legacy-001.json"),
        serde_json::to_string(&legacy).unwrap(),
    )
    .unwrap();
    fs::write(
        dir.path().join("messages").join("msg-legacy-001.body.json"),
        serde_json::to_string(&dispatch_body()).unwrap(),
    )
    .unwrap();
    let read_back = store.read("msg-legacy-001").unwrap();
    assert_eq!(read_back["task"], "implement feature");
    asep::agent_protocol::validate_dir(dir.path()).unwrap();
}

#[test]
fn test_next_id_uses_random_suffix() {
    let dir = tempfile::tempdir().unwrap();
    let store = MessageStore::new(dir.path());
    let id1 = store
        .send(
            Role::Lead,
            Role::Executor,
            Kind::TaskDispatch,
            dispatch_body(),
            None,
            None,
        )
        .unwrap()
        .id;
    let id2 = store
        .send(
            Role::Lead,
            Role::Executor,
            Kind::TaskDispatch,
            dispatch_body(),
            None,
            None,
        )
        .unwrap()
        .id;
    assert_ne!(id1, id2, "two ids must differ");
    let parse = |id: &str| -> Option<(u128, String)> {
        let rest = id.strip_prefix("msg-")?;
        let (ms, suffix) = rest.split_once('-')?;
        let ms: u128 = ms.parse().ok()?;
        Some((ms, suffix.to_string()))
    };
    let (ms1, suffix1) = parse(&id1).expect("id1 matches msg-<digits>-<hex>");
    let (_ms2, suffix2) = parse(&id2).expect("id2 matches msg-<digits>-<hex>");
    assert!(ms1 >= 1_700_000_000_000, "unix-ms prefix: {ms1}");
    assert!(
        suffix1.chars().all(|c| c.is_ascii_hexdigit()),
        "suffix must be hex: {suffix1}"
    );
    assert!(
        suffix2.chars().all(|c| c.is_ascii_hexdigit()),
        "suffix must be hex: {suffix2}"
    );
}

#[test]
fn test_task_report_optional_fields_typed() {
    let dir = tempfile::tempdir().unwrap();
    let store = MessageStore::new(dir.path());

    let mut valid = task_report_body(json!([]));
    valid
        .as_object_mut()
        .unwrap()
        .insert("tool_call_count".to_string(), json!(42));
    valid
        .as_object_mut()
        .unwrap()
        .insert("duplicate_calls".to_string(), json!(["a", "b"]));
    assert!(store
        .send(
            Role::Executor,
            Role::Leader,
            Kind::TaskReport,
            valid,
            None,
            None
        )
        .is_ok());

    let mut bad_numeric = task_report_body(json!([]));
    bad_numeric
        .as_object_mut()
        .unwrap()
        .insert("tool_call_count".to_string(), json!("lots"));
    assert!(store
        .send(
            Role::Executor,
            Role::Leader,
            Kind::TaskReport,
            bad_numeric,
            None,
            None
        )
        .is_err());

    let mut bad_array = task_report_body(json!([]));
    bad_array
        .as_object_mut()
        .unwrap()
        .insert("duplicate_calls".to_string(), json!("not an array"));
    assert!(store
        .send(
            Role::Executor,
            Role::Leader,
            Kind::TaskReport,
            bad_array,
            None,
            None
        )
        .is_err());
}

#[test]
fn test_send_sets_reply_to() {
    let dir = tempfile::tempdir().unwrap();
    let store = MessageStore::new(dir.path());
    let envelope = store
        .send(
            Role::Lead,
            Role::Executor,
            Kind::TaskDispatch,
            dispatch_body(),
            None,
            Some("msg-prev"),
        )
        .unwrap();
    assert_eq!(envelope.reply_to.as_deref(), Some("msg-prev"));
}
