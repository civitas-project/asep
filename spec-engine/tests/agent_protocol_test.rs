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
        .send(Role::Executor, Role::Lead, Kind::TaskReport, no_concerns, None)
        .is_err());
    assert!(store
        .send(
            Role::Executor,
            Role::Lead,
            Kind::TaskReport,
            task_report_body(json!([])),
            None
        )
        .is_ok());
    assert!(store
        .send(
            Role::Executor,
            Role::Lead,
            Kind::TaskReport,
            task_report_body(json!(["draft spec"])),
            None
        )
        .is_ok());
}

#[test]
fn test_lead_can_reject_own_sent_message() {
    let dir = tempfile::tempdir().unwrap();
    let store = MessageStore::new(dir.path());
    let envelope = store
        .send(Role::Lead, Role::Executor, Kind::TaskDispatch, dispatch_body(), None)
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
        .send(Role::Lead, Role::Executor, Kind::TaskDispatch, dispatch_body(), None)
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
        )
        .unwrap();
    assert!(store.ack(Role::Lead, &not_own.id, "rejected").is_err());
}

#[test]
fn test_send_writes_envelope_without_body_in_summary() {
    let dir = tempfile::tempdir().unwrap();
    let store = MessageStore::new(dir.path());
    let envelope = store
        .send(Role::Lead, Role::Executor, Kind::TaskDispatch, dispatch_body(), None)
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
    );
    assert!(result.is_err());
}

#[test]
fn test_recv_returns_only_unacked_for_recipient() {
    let dir = tempfile::tempdir().unwrap();
    let store = MessageStore::new(dir.path());
    let e1 = store
        .send(Role::Lead, Role::Executor, Kind::TaskDispatch, dispatch_body(), None)
        .unwrap();
    let e2 = store
        .send(Role::Lead, Role::Executor, Kind::TaskDispatch, dispatch_body(), None)
        .unwrap();
    let e3 = store
        .send(Role::Judge, Role::Executor, Kind::Verdict, verdict_body(), None)
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
        .send(Role::Executor, Role::Lead, Kind::TaskDispatch, dispatch_body(), None)
        .is_err());
    // executor may send TASK_REPORT per schema.
    assert!(store
        .send(
            Role::Executor,
            Role::Lead,
            Kind::TaskReport,
            task_report_body(json!([])),
            None
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
        .send(Role::Executor, Role::Lead, Kind::TaskReport, no_concerns, None)
        .is_err());
    // VERDICT rerun with non-numeric test is rejected.
    let mut bad_verdict = verdict_body();
    bad_verdict
        .as_object_mut()
        .unwrap()
        .get_mut("rerun")
        .unwrap()["test"] = json!("one");
    assert!(store
        .send(Role::Judge, Role::Lead, Kind::Verdict, bad_verdict, None)
        .is_err());
    // numeric rerun is accepted.
    assert!(store
        .send(Role::Judge, Role::Lead, Kind::Verdict, verdict_body(), None)
        .is_ok());
}
