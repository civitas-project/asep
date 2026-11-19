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
