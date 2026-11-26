//! JSON-RPC 2.0 and Model Context Protocol schema definitions.

use serde::{Deserialize, Serialize};

// @impl:FR-MCP-PROTO-01-A
// @impl:FR-MCP-PROTO-02-A

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl JsonRpcResponse {
    pub fn success(id: serde_json::Value, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: serde_json::Value, code: i64, message: &str) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.to_string(),
                data: None,
            }),
        }
    }
}

// @impl:FR-AUD-34
pub fn validate_request(value: &serde_json::Value) -> Result<JsonRpcRequest, &'static str> {
    let jsonrpc = match value.get("jsonrpc").and_then(|v| v.as_str()) {
        Some("2.0") => "2.0",
        _ => return Err("invalid_request"),
    };

    let id = match value.get("id") {
        None => None,
        Some(v) => match v {
            serde_json::Value::String(_) => Some(v.clone()),
            n @ serde_json::Value::Number(_) if n.is_i64() || n.is_u64() => Some(v.clone()),
            serde_json::Value::Null => None,
            _ => return Err("invalid_request"),
        },
    };

    let method = match value.get("method").and_then(|v| v.as_str()) {
        Some(m) if !m.is_empty() => m,
        _ => return Err("invalid_request"),
    };

    let params = value
        .get("params")
        .cloned()
        .unwrap_or(serde_json::Value::Null);

    Ok(JsonRpcRequest {
        jsonrpc: jsonrpc.to_string(),
        id,
        method: method.to_string(),
        params,
    })
}
