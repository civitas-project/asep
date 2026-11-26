// @test:FR-CAP-02
use asep::agent_protocol::Envelope;
use asep::mcp::server::process_jsonrpc_request;
use asep::mcp::tools::{execute_tool, get_tools_list};
use serde_json::json;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

// @test:FR-MCP-PROTO-01
#[test]
fn test_mcp_initialize() {
    let req = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
    let repo_path = PathBuf::from(".");
    let res = process_jsonrpc_request(req, &repo_path).expect("initialize must succeed");

    let val: serde_json::Value = serde_json::from_str(&res).unwrap();
    assert_eq!(val["jsonrpc"], "2.0");
    assert_eq!(val["id"], 1);
    assert_eq!(val["result"]["serverInfo"]["name"], "asep");
    assert!(val["result"]["capabilities"]["tools"].is_object());
}

// @test:FR-MCP-PROTO-02
#[test]
fn test_mcp_tools_list() {
    let req = r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#;
    let repo_path = PathBuf::from(".");
    let res = process_jsonrpc_request(req, &repo_path).expect("tools/list must succeed");

    let val: serde_json::Value = serde_json::from_str(&res).unwrap();
    assert_eq!(val["id"], 2);
    let tools = val["result"]["tools"].as_array().expect("tools array");
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();

    assert!(names.contains(&"asep_check"));
    assert!(names.contains(&"spec_check"));
    assert!(names.contains(&"adr_new"));
    assert!(names.contains(&"render_diagrams"));
}

// @test:FR-MCP-PROTO-COMPACT
#[test]
fn test_tool_descriptions_are_compact() {
    let tools = get_tools_list();
    let tool_list = tools["tools"].as_array().expect("tools array");
    assert_eq!(tool_list.len(), 14, "expected exactly 14 tools");
    let names: Vec<&str> = tool_list
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert!(
        names.contains(&"asep_affected"),
        "catalogue must include asep_affected"
    );
    assert!(
        names.contains(&"asep_context"),
        "catalogue must include asep_context"
    );
    for tool in tool_list {
        let name = tool["name"].as_str().unwrap_or("<unnamed>");
        let desc = tool["description"].as_str().expect("description string");
        assert!(
            desc.chars().count() <= 60,
            "tool '{name}' description too long: {} chars",
            desc.chars().count()
        );
    }
}

// @test:FR-MCP-PROTO-03
#[test]
fn test_mcp_tools_call_spec_check() {
    let req = r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"spec_check","arguments":{"dir":"."}}}"#;
    let repo_path = PathBuf::from(".");
    let res = process_jsonrpc_request(req, &repo_path).expect("tools/call must succeed");

    let val: serde_json::Value = serde_json::from_str(&res).unwrap();
    assert_eq!(val["id"], 3);
    assert!(val["result"]["isError"].is_boolean());
    let text = val["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("Traceability Gate:"));
}

// @test:FR-MCP-PROTO-04
#[test]
fn test_mcp_unknown_method_returns_error_frame() {
    let req = r#"{"jsonrpc":"2.0","id":99,"method":"unknown/method","params":{}}"#;
    let repo_path = PathBuf::from(".");
    let res = process_jsonrpc_request(req, &repo_path).expect("must return error frame");

    let val: serde_json::Value = serde_json::from_str(&res).unwrap();
    assert_eq!(val["id"], 99);
    assert_eq!(val["error"]["code"], -32601); // Method not found
}

// @test:FR-MCP-PROTO-05
#[test]
fn test_mcp_tools_list_includes_srcidx_and_srcdump() {
    let req = r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#;
    let repo_path = PathBuf::from(".");
    let res = process_jsonrpc_request(req, &repo_path).expect("tools/list must succeed");

    let val: serde_json::Value = serde_json::from_str(&res).unwrap();
    assert_eq!(val["id"], 2);
    let tools = val["result"]["tools"].as_array().expect("tools array");
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();

    assert!(names.contains(&"srcidx"), "srcidx tool should be listed");
    assert!(names.contains(&"srcdump"), "srcdump tool should be listed");
}

// @test:FR-MCP-PROTO-06
#[test]
fn test_mcp_tools_call_srcidx() {
    let req = r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"srcidx","arguments":{"dir":".","mode":"symbols-only"}}}"#;
    let repo_path = PathBuf::from(".");
    let res = process_jsonrpc_request(req, &repo_path).expect("tools/call srcidx must succeed");

    let val: serde_json::Value = serde_json::from_str(&res).unwrap();
    assert_eq!(val["id"], 5);
    assert!(val["result"]["isError"].is_boolean());
    let text = val["result"]["content"][0]["text"].as_str().unwrap();

    // Should be valid NDJSON with header
    let lines: Vec<&str> = text.lines().collect();
    assert!(!lines.is_empty(), "srcidx should produce output");

    // First line should be header with schema
    let header: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(header["schema"], "srcdump.v1");
    assert!(header["files"].is_number());
}

// @test:FR-MCP-PROTO-07
#[test]
fn test_mcp_tools_call_srcdump() {
    let req = r#"{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"srcdump","arguments":{"dir":".","mode":"full","format":"ndjson"}}}"#;
    let repo_path = PathBuf::from(".");
    let res = process_jsonrpc_request(req, &repo_path).expect("tools/call srcdump must succeed");

    let val: serde_json::Value = serde_json::from_str(&res).unwrap();
    assert_eq!(val["id"], 6);
    assert!(val["result"]["isError"].is_boolean());
    let text = val["result"]["content"][0]["text"].as_str().unwrap();

    // Should be valid NDJSON with header
    let lines: Vec<&str> = text.lines().collect();
    assert!(!lines.is_empty(), "srcdump should produce output");

    // First line should be header with schema
    let header: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(header["schema"], "srcdump.v1");
    assert!(header["files"].is_number());

    // Should have at least one file entry
    if lines.len() > 1 {
        let entry: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(entry["type"], "file");
        assert!(entry["path"].is_string());
        assert!(entry["lang"].is_string());
    }
}

// @test:FR-MCP-PROTO-08
#[test]
fn test_agent_tools_do_not_return_payload() {
    let dir = tempfile::tempdir().unwrap();
    let run_dir = dir.path().to_path_buf();
    let body = json!({
        "task": "implement feature",
        "files": ["src/feature.rs"],
        "acceptance": "feature works",
        "verification_command": "cargo test -p asep"
    });
    let res = execute_tool(
        "asep_agent_send",
        &json!({
            "run_dir": run_dir.to_str().unwrap(),
            "agent": "leader",
            "to": "executor",
            "kind": "TASK_DISPATCH",
            "body": body,
        }),
        Path::new(""),
    )
    .expect("asep_agent_send must succeed");

    let text = res["content"][0]["text"].as_str().expect("text content");
    let val: serde_json::Value = serde_json::from_str(text).expect("response text is JSON");
    assert!(val["id"].is_string(), "send response must include id");
    assert!(val["path"].is_string(), "send response must include path");
    let path = val["path"].as_str().unwrap();
    assert!(path.starts_with("messages/") && path.ends_with(".json"));
    assert!(
        !val.as_object().unwrap().contains_key("body"),
        "send must not return body payload"
    );
}

// @test:FR-MCP-PROTO-09
#[test]
fn test_agent_recv_filters_by_agent() {
    let dir = tempfile::tempdir().unwrap();
    let run_dir = dir.path().to_path_buf();
    let run_dir_s = run_dir.to_str().unwrap().to_string();
    let dispatch = json!({
        "task": "implement feature",
        "files": ["src/feature.rs"],
        "acceptance": "feature works",
        "verification_command": "cargo test -p asep"
    });
    let ruling = json!({
        "finding": "ok",
        "decision": "approve",
        "cost_if_wrong": 0
    });

    execute_tool(
        "asep_agent_send",
        &json!({
            "run_dir": run_dir_s,
            "agent": "leader",
            "to": "executor",
            "kind": "TASK_DISPATCH",
            "body": dispatch,
        }),
        Path::new(""),
    )
    .expect("send to executor must succeed");
    execute_tool(
        "asep_agent_send",
        &json!({
            "run_dir": run_dir_s,
            "agent": "leader",
            "to": "judge",
            "kind": "RULING",
            "body": ruling,
        }),
        Path::new(""),
    )
    .expect("send to judge must succeed");

    let res = execute_tool(
        "asep_agent_recv",
        &json!({"run_dir": run_dir_s, "agent": "executor"}),
        Path::new(""),
    )
    .expect("recv must succeed");
    let text = res["content"][0]["text"].as_str().expect("text content");
    let list: Vec<serde_json::Value> = serde_json::from_str(text).expect("recv list is JSON");
    assert_eq!(
        list.len(),
        1,
        "executor inbox must exclude messages to other agents"
    );
    assert!(list[0]["id"].is_string());
    assert!(list[0]["path"].is_string());
    assert!(
        !list[0].as_object().unwrap().contains_key("body"),
        "recv must not return body payload"
    );
}

// @test:FR-MCP-PROTO-10
#[test]
fn test_agent_send_sets_reply_to() {
    let dir = tempfile::tempdir().unwrap();
    let run_dir = dir.path().to_path_buf();
    let run_dir_s = run_dir.to_str().unwrap().to_string();
    let body = json!({
        "task": "implement feature",
        "files": ["src/feature.rs"],
        "acceptance": "feature works",
        "verification_command": "cargo test -p asep"
    });

    let res = execute_tool(
        "asep_agent_send",
        &json!({
            "run_dir": run_dir_s,
            "agent": "leader",
            "to": "executor",
            "kind": "TASK_DISPATCH",
            "body": body,
            "reply_to": "msg-prev",
        }),
        Path::new(""),
    )
    .expect("asep_agent_send must succeed");

    let text = res["content"][0]["text"].as_str().expect("text content");
    let val: serde_json::Value = serde_json::from_str(text).expect("response text is JSON");
    let id = val["id"].as_str().expect("id is string");

    // Raw envelope file must use semantic key "reply_to".
    let raw = fs::read_to_string(dir.path().join("messages").join(format!("{id}.json"))).unwrap();
    assert!(
        raw.contains("\"reply_to\":\"msg-prev\""),
        "raw envelope must contain reply_to key with msg-prev value"
    );

    // Deserialized envelope must expose reply_to.
    let envelope: Envelope = serde_json::from_str(&raw).unwrap();
    assert_eq!(envelope.reply_to.as_deref(), Some("msg-prev"));
}
// @test:FR-CAP-03
#[test]
fn test_agent_send_blocks_unsupported_media() {
    let run_dir = tempfile::tempdir().unwrap();
    let default_dir = tempfile::tempdir().unwrap();
    let run_dir_s = run_dir.path().to_str().unwrap().to_string();

    let res = execute_tool(
        "asep_agent_send",
        &json!({
            "run_dir": run_dir_s,
            "agent": "leader",
            "to": "executor",
            "kind": "TASK_DISPATCH",
            "body": json!({"task": "review image"}),
            "media": "image",
        }),
        default_dir.path(),
    )
    .expect("asep_agent_send must return");

    assert!(res["isError"].as_bool().unwrap(), "expected error: {res:?}");
    let text = res["content"][0]["text"].as_str().expect("text content");
    let val: serde_json::Value = serde_json::from_str(text).expect("response text is JSON");
    assert_eq!(val["blocked"], true);
    assert!(
        val["reason"]
            .as_str()
            .unwrap()
            .starts_with("BLOCKED: requires media:image"),
        "unexpected reason: {:?}",
        val["reason"]
    );

    // No message file must be written when blocked.
    assert!(!run_dir.path().join("messages").exists());
}

// @test:FR-CAP-03
#[test]
fn test_agent_send_allows_supported_media() {
    let run_dir = tempfile::tempdir().unwrap();
    let default_dir = tempfile::tempdir().unwrap();
    let run_dir_s = run_dir.path().to_str().unwrap().to_string();

    let res = execute_tool(
        "asep_agent_send",
        &json!({
            "run_dir": run_dir_s,
            "agent": "leader",
            "to": "executor",
            "kind": "TASK_DISPATCH",
            "body": json!({
                "task": "review text",
                "files": [],
                "acceptance": "ok",
                "verification_command": "cargo test -p asep"
            }),
            "media": "text",
        }),
        default_dir.path(),
    )
    .expect("asep_agent_send must succeed");

    assert_eq!(res["isError"].as_bool(), Some(false));
    let text = res["content"][0]["text"].as_str().expect("text content");
    let val: serde_json::Value = serde_json::from_str(text).expect("response text is JSON");
    let id = val["id"].as_str().expect("id is string");
    assert!(
        run_dir
            .path()
            .join("messages")
            .join(format!("{id}.json"))
            .exists(),
        "message file must be written"
    );
}

// @test:FR-CAP-08
#[test]
fn test_agent_send_blocks_missing_required_capability() {
    let run_dir = tempfile::tempdir().unwrap();
    let default_dir = tempfile::tempdir().unwrap();
    let run_dir_s = run_dir.path().to_str().unwrap().to_string();

    let config_dir = default_dir.path().join(".spec-engine");
    fs::create_dir_all(&config_dir).unwrap();
    fs::write(
        config_dir.join("config.toml"),
        "[capabilities]\nweb = false\n",
    )
    .unwrap();

    let res = execute_tool(
        "asep_agent_send",
        &json!({
            "run_dir": run_dir_s,
            "agent": "leader",
            "to": "executor",
            "kind": "TASK_DISPATCH",
            "body": json!({"task": "fetch"}),
            "required_capabilities": ["web"],
        }),
        default_dir.path(),
    )
    .expect("asep_agent_send must return");

    assert!(res["isError"].as_bool().unwrap(), "expected error: {res:?}");
    let text = res["content"][0]["text"].as_str().expect("text content");
    let val: serde_json::Value = serde_json::from_str(text).expect("response text is JSON");
    assert_eq!(val["blocked"], true);
    assert!(
        val["reason"]
            .as_str()
            .unwrap()
            .starts_with("BLOCKED: requires web"),
        "unexpected reason: {:?}",
        val["reason"]
    );

    // No message file must be written when blocked.
    assert!(!run_dir.path().join("messages").exists());
}

// @test:FR-MCP-03
#[test]
fn test_accept_scoped_tool_listed() {
    let tools = get_tools_list();
    let tool_list = tools["tools"].as_array().expect("tools array");
    let tool = tool_list
        .iter()
        .find(|t| t["name"] == "asep_accept_scoped")
        .expect("asep_accept_scoped must be listed");
    assert_eq!(
        tool["description"].as_str().unwrap(),
        "Runs scoped non-root acceptance via the trusted runner."
    );
    let props = tool["inputSchema"]["properties"]
        .as_object()
        .expect("properties object");
    assert!(props.contains_key("commit"));
    assert!(props.contains_key("dir"));
    let required: Vec<&str> = tool["inputSchema"]["required"]
        .as_array()
        .expect("required array")
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert!(required.contains(&"commit"), "commit is required");
    assert!(!required.contains(&"dir"), "dir is optional");
}

// @test:FR-MCP-03
#[test]
fn test_accept_scoped_rejects_invalid_commit() {
    let dir = tempfile::tempdir().unwrap();
    let dir_s = dir.path().to_str().unwrap().to_string();
    for commit in [
        "short",
        "gggggggggggggggggggggggggggggggggggggggg",
        "ABCDEF0123456789ABCDEF0123456789ABCDEF01",
    ] {
        let res = execute_tool(
            "asep_accept_scoped",
            &json!({"commit": commit, "dir": dir_s}),
            Path::new(""),
        )
        .expect("asep_accept_scoped must return");
        assert!(
            res["isError"].as_bool() == Some(true),
            "expected isError=true for commit '{commit}': {res:?}"
        );
        let text = res["content"][0]["text"].as_str().expect("text content");
        assert!(text.contains("commit"), "unclear diagnostic: {text}");
    }
}

// @test:FR-MCP-03
#[test]
fn test_accept_scoped_success_writes_evidence() {
    let root = tempfile::tempdir().expect("temp dir");
    let root_path = root.path();

    // Minimal temp git repo.
    git(root_path, &["init", "-q", root_path.to_str().unwrap()]);
    Command::new("git")
        .arg("-C")
        .arg(root_path)
        .arg("config")
        .arg("user.email")
        .arg("test@example.com")
        .status()
        .expect("git config email");
    Command::new("git")
        .arg("-C")
        .arg(root_path)
        .arg("config")
        .arg("user.name")
        .arg("test")
        .status()
        .expect("git config name");
    fs::write(root_path.join(".gitignore"), "acceptance.json\nevidence/\n")
        .expect("write .gitignore");
    fs::write(root_path.join("README.md"), "# hello\n").expect("write README");
    fs::create_dir_all(root_path.join("docs")).expect("docs dir");
    fs::write(
        root_path.join("docs/PRD.md"),
        "# PRD\n\n@req:REQ-ALPHA:MUST\n",
    )
    .expect("write PRD");

    git(
        root_path,
        &["add", ".gitignore", "README.md", "docs/PRD.md"],
    );
    git(root_path, &["commit", "-q", "-m", "init"]);

    let output = Command::new("git")
        .arg("-C")
        .arg(root_path)
        .arg("rev-parse")
        .arg("HEAD")
        .output()
        .expect("git rev-parse HEAD");
    let commit = String::from_utf8(output.stdout)
        .expect("commit sha is utf-8")
        .trim()
        .to_string();

    // Policy + PRD with one MUST requirement.
    fs::create_dir_all(root_path.join(".spec-engine")).expect("spec-engine dir");
    let policy = format!(
        r#"{{"approved_commit":"{commit}","changed_paths":["docs/PRD.md"],"evidence_dir":"evidence"}}"#
    );
    fs::write(root_path.join(".spec-engine/acceptance.json"), policy).expect("write policy");

    // Fake bwrap shim: every gate exits 0 without running anything.
    let shim_path = bwrap_shim();
    std::env::set_var("ASEP_BWRAP", shim_path.to_str().unwrap());

    let res = execute_tool(
        "asep_accept_scoped",
        &json!({"commit": commit, "dir": root_path.to_str().unwrap()}),
        Path::new(""),
    )
    .expect("asep_accept_scoped must return");

    assert_eq!(
        res["isError"].as_bool(),
        Some(false),
        "unexpected error: {res:?}"
    );
    let text = res["content"][0]["text"].as_str().expect("text content");
    let evidence_dir = root_path.join("evidence");
    assert!(
        text.contains(evidence_dir.to_str().unwrap()),
        "response text must name evidence dir: {text}"
    );

    let semantic = evidence_dir.join("semantic");
    let evidence = semantic.join("REQ-ALPHA.md");
    assert!(evidence.is_file(), "missing evidence for REQ-ALPHA");
    let content = fs::read_to_string(&evidence).unwrap();
    assert!(content.contains("reviewer: scoped-trusted-runner"));
    assert!(content.contains("verdict: ACCEPTED"));
}

// @test:IN-01-REPORT
#[test]
fn test_trace_report_tool_listed() {
    let tools = get_tools_list();
    let tool_list = tools["tools"].as_array().expect("tools array");
    let tool = tool_list
        .iter()
        .find(|t| t["name"] == "asep_trace_report")
        .expect("asep_trace_report must be listed");
    let desc = tool["description"].as_str().expect("description string");
    assert!(desc.chars().count() <= 60, "description too long: {desc}");
    let props = tool["inputSchema"]["properties"]
        .as_object()
        .expect("properties object");
    assert!(props.contains_key("trace_id"));
    assert!(props.contains_key("dir"));
    assert_eq!(props["trace_id"]["type"], "string");
    assert_eq!(props["dir"]["type"], "string");
    let required: Vec<&str> = tool["inputSchema"]["required"]
        .as_array()
        .expect("required array")
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert!(required.contains(&"trace_id"), "trace_id is required");
    assert!(!required.contains(&"dir"), "dir is optional");
}

// @test:IN-01-REPORT
#[test]
fn test_trace_report_rejects_invalid_trace_id() {
    let dir = tempfile::tempdir().unwrap();
    let dir_s = dir.path().to_str().unwrap().to_string();
    for trace_id in ["", "../escape", "bad id", "a/b"] {
        let res = execute_tool(
            "asep_trace_report",
            &json!({"trace_id": trace_id, "dir": dir_s}),
            Path::new(""),
        )
        .expect("asep_trace_report must return");
        assert!(
            res["isError"].as_bool() == Some(true),
            "expected isError=true for trace_id '{trace_id}': {res:?}"
        );
    }
}

// @test:IN-01-REPORT
#[test]
fn test_trace_report_success_returns_summary() {
    let dir = tempfile::tempdir().unwrap();
    let dir_path = dir.path();
    let trace_dir = dir_path.join(".spec-engine").join("trace");
    fs::create_dir_all(&trace_dir).unwrap();

    let trace_id = "trace.alpha-1";
    fs::write(
        trace_dir.join(format!("{trace_id}.ndjson")),
        r#"{"task_id":"task-alpha","event":"run"}
{"task_id":"task-alpha","event":"finish"}
"#,
    )
    .unwrap();
    fs::write(
        dir_path.join(".spec-engine").join("verification.json"),
        r#"{"status":"reconciled"}"#,
    )
    .unwrap();

    let res = execute_tool(
        "asep_trace_report",
        &json!({"trace_id": trace_id, "dir": dir_path.to_str().unwrap()}),
        Path::new(""),
    )
    .expect("asep_trace_report must return");

    assert_eq!(
        res["isError"].as_bool(),
        Some(false),
        "unexpected error: {res:?}"
    );
    let text = res["content"][0]["text"].as_str().expect("text content");
    let summary: serde_json::Value = serde_json::from_str(text).expect("response text is JSON");
    assert_eq!(
        summary["trace_found"].as_bool(),
        Some(true),
        "trace must be found: {text}"
    );
    assert_eq!(summary["reconciliation"].as_str(), Some("reconciled"));
    assert!(
        text.contains("task-alpha"),
        "summary must name task_id: {text}"
    );
    assert!(
        text.contains("\"run\""),
        "summary must name run event: {text}"
    );
    assert!(
        text.contains("\"finish\""),
        "summary must name finish event: {text}"
    );
    assert!(
        !text.contains("\"isError\": true"),
        "read-only report must not be an error: {text}"
    );
}

// @test:IN-01-REPORT
#[test]
fn test_trace_report_missing_trace_is_not_error() {
    let dir = tempfile::tempdir().unwrap();
    let dir_s = dir.path().to_str().unwrap().to_string();

    let res = execute_tool(
        "asep_trace_report",
        &json!({"trace_id": "trace.does-not-exist", "dir": dir_s}),
        Path::new(""),
    )
    .expect("asep_trace_report must return");

    assert_eq!(res["isError"].as_bool(), Some(false));
    let text = res["content"][0]["text"].as_str().expect("text content");
    let summary: serde_json::Value = serde_json::from_str(text).expect("response text is JSON");
    assert_eq!(summary["trace_found"].as_bool(), Some(false));
    assert!(summary["events"].as_array().unwrap().is_empty());
}

// @test:FR-MCP-IMPACT-01
#[test]
fn test_asep_affected_returns_structured_report() {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = dir.path();
    let root_s = root.to_str().unwrap().to_string();

    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("src/engine.rs"),
        "// @impl:IMP-ALPHA\nfn process() {}\n",
    )
    .unwrap();
    fs::create_dir_all(root.join("reqs")).unwrap();
    fs::write(
        root.join("reqs/REQ-ALPHA.md"),
        "# Requirement\n@req:REQ-ALPHA:MUST\n",
    )
    .unwrap();
    fs::create_dir_all(root.join("tests")).unwrap();
    fs::write(
        root.join("tests/engine_test.rs"),
        "#[test]\nfn check_process() {}\n// @test:TEST-BETA\n",
    )
    .unwrap();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(root.join("docs/ADR-1.md"), "# ADR-1\n@adr:ADR-1\n").unwrap();
    fs::write(
        root.join("spec.rel"),
        "@rel:satisfies:IMP-ALPHA->REQ-ALPHA \"impl satisfies req\"\n\
         @rel:verifies:TEST-BETA->REQ-ALPHA \"test verifies req\"\n\
         @rel:informs:REQ-ALPHA->ADR-1 \"req informs adr\"\n",
    )
    .unwrap();
    fs::write(root.join("notes.txt"), "plain notes, no tags\n").unwrap();

    let res = execute_tool(
        "asep_affected",
        &json!({
            "paths": ["src/engine.rs", "notes.txt"],
            "root": root_s,
        }),
        Path::new(""),
    )
    .expect("asep_affected must return");

    assert_eq!(
        res["isError"].as_bool(),
        Some(false),
        "unexpected error: {res:?}"
    );
    let text = res["content"][0]["text"].as_str().expect("text content");
    let report: serde_json::Value = serde_json::from_str(text).expect("response text is JSON");

    let ids = |field: &str| -> Vec<String> {
        report[field]
            .as_array()
            .unwrap_or_else(|| panic!("missing array field '{field}': {text}"))
            .iter()
            .filter_map(|v| v["id"].as_str().map(String::from))
            .collect()
    };
    let impl_ids = ids("impls");
    let req_ids = ids("requirements");
    let test_ids = ids("tests");
    let adr_ids = ids("adrs");

    assert!(
        impl_ids.contains(&"IMP-ALPHA".to_string()),
        "impls must include IMP-ALPHA: {impl_ids:?}"
    );
    assert!(
        req_ids.contains(&"REQ-ALPHA".to_string()),
        "requirements must include REQ-ALPHA: {req_ids:?}"
    );
    assert!(
        test_ids.contains(&"TEST-BETA".to_string()),
        "tests must include TEST-BETA: {test_ids:?}"
    );
    assert!(
        adr_ids.contains(&"ADR-1".to_string()),
        "adrs must include ADR-1: {adr_ids:?}"
    );

    let unmapped: Vec<String> = report["unmapped"]
        .as_array()
        .unwrap_or_else(|| panic!("missing array field 'unmapped': {text}"))
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect();
    assert!(
        unmapped.contains(&"notes.txt".to_string()),
        "unmapped must list notes.txt: {unmapped:?}"
    );

    let provenance = report["provenance"]
        .as_object()
        .unwrap_or_else(|| panic!("missing object field 'provenance': {text}"));
    assert!(!provenance.is_empty(), "provenance must be non-empty");

    assert!(
        report["commands"]
            .as_array()
            .is_some_and(|cmds| !cmds.is_empty()),
        "commands must be non-empty: {text}"
    );
}

// @test:FR-MCP-IMPACT-02
#[test]
fn test_asep_affected_rejects_empty_and_missing_paths() {
    let dir = tempfile::tempdir().expect("temp dir");
    let root_s = dir.path().to_str().unwrap().to_string();

    for args in [json!({}), json!({"paths": [], "root": root_s})] {
        let res = execute_tool("asep_affected", &args, Path::new(""));
        assert!(
            res.is_err(),
            "asep_affected must error on empty/missing paths: {res:?}"
        );
    }
}

// @test:FR-CTX-03
#[test]
fn test_asep_context_returns_bundle() {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = dir.path();
    let root_s = root.to_str().unwrap().to_string();

    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("src/engine.rs"),
        "// @impl:IMP-ALPHA\nfn process() {}\n",
    )
    .unwrap();
    fs::create_dir_all(root.join("reqs")).unwrap();
    fs::write(
        root.join("reqs/REQ-ALPHA.md"),
        "# Requirement\n@req:REQ-ALPHA:MUST\n",
    )
    .unwrap();

    let res = execute_tool(
        "asep_context",
        &json!({
            "paths": ["src/engine.rs"],
            "root": root_s,
        }),
        Path::new(""),
    )
    .expect("asep_context must return");

    assert_eq!(
        res["isError"].as_bool(),
        Some(false),
        "unexpected error: {res:?}"
    );
    let text = res["content"][0]["text"].as_str().expect("text content");
    let bundle: serde_json::Value = serde_json::from_str(text).expect("response text is JSON");

    assert!(
        bundle["objective"].is_string(),
        "objective must be string: {text}"
    );
    assert!(
        bundle["impacted"].is_object(),
        "impacted must be object: {text}"
    );
    assert!(
        bundle["excerpts"].is_array(),
        "excerpts must be array: {text}"
    );
    assert!(
        bundle["truncation"].is_string(),
        "truncation must be string: {text}"
    );
    assert!(
        bundle["commands"].is_array(),
        "commands must be array: {text}"
    );
}

#[test]
fn test_asep_context_rejects_empty_and_missing_paths() {
    let dir = tempfile::tempdir().expect("temp dir");
    let root_s = dir.path().to_str().unwrap().to_string();

    for args in [json!({}), json!({"paths": [], "root": root_s})] {
        let res = execute_tool("asep_context", &args, Path::new(""));
        assert!(
            res.is_err(),
            "asep_context must error on empty/missing paths: {res:?}"
        );
    }
}

// @test:FR-AUD-34
#[test]
fn test_fr_aud_34_jsonrpc_error_categories() {
    let repo_path = PathBuf::from(".");

    let assert_code = |req: &str, code: i64| {
        let res = process_jsonrpc_request(req, &repo_path).expect("must return an error frame");
        let val: serde_json::Value = serde_json::from_str(&res).unwrap();
        assert_eq!(val["error"]["code"], code);
    };

    assert_code("{broken json", -32700);
    assert_code(r#"{"jsonrpc":"1.0","id":1,"method":"initialize"}"#, -32600);
    assert_code(
        r#"{"jsonrpc":"2.0","id":{"foo":1},"method":"ping"}"#,
        -32600,
    );
    assert_code(r#"{"jsonrpc":"2.0","id":[1],"method":"ping"}"#, -32600);
    assert_code(
        r#"{"jsonrpc":"2.0","id":7,"method":"no/such.method","params":{}}"#,
        -32601,
    );
    assert_code(
        r#"{"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"does-not-exist-tool"}}"#,
        -32602,
    );

    let res = process_jsonrpc_request(r#"{"jsonrpc":"2.0","method":"ping"}"#, &repo_path);
    assert!(res.is_none(), "notification must not produce a response");
}

fn git(root: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(root)
        .status()
        .unwrap_or_else(|e| panic!("failed to run git {args:?}: {e}"));
    assert!(status.success(), "git {args:?} failed");
}

/// Fake bwrap shim: every invocation exits 0 without running anything, so
/// all scoped gates "pass" without a real sandbox.
fn bwrap_shim() -> PathBuf {
    let shim_path = std::env::temp_dir().join("asep-mcp-test-bwrap");
    let mut shim = fs::File::create(&shim_path).expect("create shim");
    writeln!(
        shim,
        r#"#!/bin/sh
# Fake bwrap shim: scoped gates exit 0. `asep affected --json` returns a plan
# that covers the PRD requirement; `sh -c` runs report one passing test.
case "$*" in
    *affected*--json*)
        echo '{{"requirements":[{{"id":"REQ-ALPHA"}}],"commands":["cargo test -p asep --test mcp_test -- --test-threads=1 --skip test_mcp_tools_call_srcdump --skip test_mcp_tools_call_srcidx"]}}'
        ;;
    *)
        echo "test result: ok. 1 passed"
        ;;
esac
exit 0
"#
    )
    .expect("write shim");
    drop(shim);
    let mut perms = fs::metadata(&shim_path)
        .expect("shim metadata")
        .permissions();
    use std::os::unix::fs::PermissionsExt;
    perms.set_mode(0o755);
    fs::set_permissions(&shim_path, perms).expect("shim perms");
    shim_path
}
