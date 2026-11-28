//! Standard I/O loop and JSON-RPC message processing for MCP.

use crate::mcp::protocol::{validate_request, JsonRpcRequest, JsonRpcResponse};
use crate::mcp::tools::{execute_tool, get_tools_list};
use std::io::{BufRead, Write};
use std::path::Path;

// @impl:FR-MCP-PROTO-01-B @impl:FR-MCP-01
// @impl:FR-MCP-PROTO-02-C
// @impl:FR-MCP-PROTO-03-B
// @impl:FR-MCP-PROTO-04

/// Protocol versions this adapter can answer. A client may request one of
/// these; unsupported versions are reported honestly rather than silently
/// accepted. A native-client pilot is still required before claiming full
/// interoperability.
const SUPPORTED_PROTOCOL_VERSIONS: [&str; 2] = ["2024-11-05", "2025-03-26"];
const DEFAULT_PROTOCOL_VERSION: &str = "2024-11-05";

fn negotiated_version(req: &JsonRpcRequest) -> String {
    req.params
        .get("protocolVersion")
        .and_then(|v| v.as_str())
        .filter(|v| SUPPORTED_PROTOCOL_VERSIONS.contains(v))
        .unwrap_or(DEFAULT_PROTOCOL_VERSION)
        .to_string()
}

/// Minimal default capability exposure. There is no legacy full-surface
/// mode: `ASEP_MCP_TOOLS` replaces the default with an explicit list.
const DEFAULT_MCP_TOOLS: [&str; 4] = ["asep_check", "spec_check", "asep_context", "asep_affected"];

fn tool_exposed(name: &str) -> bool {
    match std::env::var("ASEP_MCP_TOOLS") {
        Ok(allowlist) => allowlist.split(',').map(str::trim).any(|allowed| allowed == name),
        Err(_) => DEFAULT_MCP_TOOLS.contains(&name),
    }
}

/// Maps an error category to its JSON-RPC error code.
fn error_code(category: &str) -> i64 {
    match category {
        "parse" => -32700,
        "invalid_request" => -32600,
        "method_not_found" => -32601,
        "invalid_params" => -32602,
        "tool_error" => -32000,
        _ => -32600,
    }
}

/// Returns the echoable request id (string or integer), else JSON null.
fn request_id_value(value: &serde_json::Value) -> serde_json::Value {
    match value.get("id") {
        Some(v) => match v {
            serde_json::Value::String(_) => v.clone(),
            n @ serde_json::Value::Number(_) if n.is_i64() || n.is_u64() => v.clone(),
            _ => serde_json::Value::Null,
        },
        None => serde_json::Value::Null,
    }
}

/// Builds an error response frame for a failed validation/category.
fn error_response(value: &serde_json::Value, category: &str) -> Option<String> {
    let code = error_code(category);
    let id = request_id_value(value);
    let err = JsonRpcResponse::error(id, code, category);
    serde_json::to_string(&err).ok()
}

/// Dispatches a validated request to its method handler.
fn handle_method(
    req: &JsonRpcRequest,
    default_dir: &Path,
) -> Result<serde_json::Value, &'static str> {
    match req.method.as_str() {
        "initialize" => Ok(serde_json::json!({
            "protocolVersion": negotiated_version(req),
            "supportedVersions": SUPPORTED_PROTOCOL_VERSIONS,
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": "asep",
                "version": env!("CARGO_PKG_VERSION")
            }
        })),
        "tools/list" => {
            let mut list = get_tools_list();
            if let Some(tools) = list.get_mut("tools").and_then(|v| v.as_array_mut()) {
                tools.retain(|tool| {
                    tool.get("name")
                        .and_then(|v| v.as_str())
                        .is_some_and(tool_exposed)
                });
            }
            Ok(list)
        }
        "ping" => Ok(serde_json::json!({})),
        "tools/call" => {
            let name = req
                .params
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let known = get_tools_list()
                .get("tools")
                .and_then(|v| v.as_array())
                .is_some_and(|tools| {
                    tools
                        .iter()
                        .any(|tool| tool.get("name").and_then(|v| v.as_str()) == Some(name))
                });
            if !known || !tool_exposed(name) {
                return Err("invalid_params");
            }
            let empty_obj = serde_json::json!({});
            let arguments = req.params.get("arguments").unwrap_or(&empty_obj);

            execute_tool(name, arguments, default_dir).map_err(|_| "tool_error")
        }
        _ => Err("method_not_found"),
    }
}

/// Processes a single JSON-RPC string message and returns the response string if one is required.
pub fn process_jsonrpc_request(raw_json: &str, default_dir: &Path) -> Option<String> {
    let value: serde_json::Value = match serde_json::from_str(raw_json) {
        Ok(v) => v,
        Err(_) => {
            let err = JsonRpcResponse::error(serde_json::Value::Null, -32700, "Parse error");
            return serde_json::to_string(&err).ok();
        }
    };

    let req = match validate_request(&value) {
        Ok(r) => r,
        Err(category) => return error_response(&value, category),
    };

    let id = match &req.id {
        Some(id_val) => id_val.clone(),
        None => {
            // Notification: still dispatched to the method handler; no response is sent.
            let _ = handle_method(&req, default_dir);
            return None;
        }
    };

    let res = match handle_method(&req, default_dir) {
        Ok(result) => JsonRpcResponse::success(id, result),
        Err(category) => JsonRpcResponse::error(id, error_code(category), category),
    };

    serde_json::to_string(&res).ok()
}

/// Runs the standard I/O loop reading JSON-RPC requests from stdin and printing responses to stdout.
pub fn run_mcp_server(default_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    for line in stdin.lock().lines() {
        let line = line?;

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
