use asep::mcp::server::process_jsonrpc_request;
use asep::mcp::tools::execute_tool;
use serde_json::json;
use std::path::{Path, PathBuf};

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
            "agent": "lead",
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
            "agent": "lead",
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
            "agent": "lead",
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
    assert_eq!(list.len(), 1, "executor inbox must exclude messages to other agents");
    assert!(list[0]["id"].is_string());
    assert!(list[0]["path"].is_string());
    assert!(
        !list[0].as_object().unwrap().contains_key("body"),
        "recv must not return body payload"
    );
}
