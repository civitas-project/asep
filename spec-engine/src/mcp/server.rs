//! Standard I/O loop and JSON-RPC message processing for MCP.

use crate::mcp::protocol::{JsonRpcRequest, JsonRpcResponse};
use crate::mcp::tools::{execute_tool, get_tools_list};
use std::io::{BufRead, Write};
use std::path::Path;

// @impl:FR-MCP-PROTO-01
// @impl:FR-MCP-PROTO-02
// @impl:FR-MCP-PROTO-03
// @impl:FR-MCP-PROTO-04

/// Processes a single JSON-RPC string message and returns the response string if one is required.
pub fn process_jsonrpc_request(raw_json: &str, default_dir: &Path) -> Option<String> {
    let req: JsonRpcRequest = match serde_json::from_str(raw_json) {
        Ok(r) => r,
        Err(_) => {
            let err = JsonRpcResponse::error(serde_json::Value::Null, -32700, "Parse error");
            return serde_json::to_string(&err).ok();
        }
    };

    let id = match req.id {
        Some(id_val) => id_val,
        None => {
            // Notification: No response is sent
            return None;
        }
    };

    let res = match req.method.as_str() {
        "initialize" => JsonRpcResponse::success(
            id,
            serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "asep",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }),
        ),

        "tools/list" => JsonRpcResponse::success(id, get_tools_list()),

        "tools/call" => {
            let name = req
                .params
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let empty_obj = serde_json::json!({});
            let arguments = req.params.get("arguments").unwrap_or(&empty_obj);

            match execute_tool(name, arguments, default_dir) {
                Ok(content_val) => JsonRpcResponse::success(id, content_val),
                Err(err_msg) => JsonRpcResponse::error(id, -32603, &err_msg),
            }
        }

        "ping" => JsonRpcResponse::success(id, serde_json::json!({})),

        _ => JsonRpcResponse::error(id, -32601, "Method not found"),
    };

    serde_json::to_string(&res).ok()
}

/// Runs the standard I/O loop reading JSON-RPC requests from stdin and printing responses to stdout.
pub fn run_mcp_server(default_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break, // EOF or IO error
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(res) = process_jsonrpc_request(trimmed, default_dir) {
            writeln!(stdout, "{}", res)?;
            stdout.flush()?;
        }
    }

    Ok(())
}
