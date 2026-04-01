//! JSON-RPC 2.0 and MCP protocol types.

use serde::{Deserialize, Serialize};

/// A JSON-RPC 2.0 request.
#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcRequest {
    /// Always `"2.0"`.
    pub jsonrpc: String,
    /// Unique request id.
    pub id: u64,
    /// The method to invoke.
    pub method: String,
    /// Optional parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

impl JsonRpcRequest {
    /// Build a new request with the given id, method, and optional params.
    #[must_use]
    pub fn new(id: u64, method: &str, params: Option<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_owned(),
            id,
            method: method.to_owned(),
            params,
        }
    }
}

/// A JSON-RPC 2.0 notification (no `id`).
#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcNotification {
    /// Always `"2.0"`.
    pub jsonrpc: String,
    /// The notification method.
    pub method: String,
    /// Optional parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

impl JsonRpcNotification {
    /// Build a new notification.
    #[must_use]
    pub fn new(method: &str, params: Option<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_owned(),
            method: method.to_owned(),
            params,
        }
    }
}

/// A JSON-RPC 2.0 response.
#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcResponse {
    /// Protocol version.
    pub jsonrpc: String,
    /// Matches the request id; absent for notifications.
    pub id: Option<u64>,
    /// Successful result payload.
    pub result: Option<serde_json::Value>,
    /// Error payload.
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC 2.0 error object.
#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcError {
    /// Numeric error code.
    pub code: i64,
    /// Human-readable description.
    pub message: String,
    /// Optional structured data.
    pub data: Option<serde_json::Value>,
}

/// Server capabilities returned by `initialize`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ServerCapabilities {
    /// Tool-related capabilities.
    pub tools: Option<serde_json::Value>,
    /// Resource-related capabilities.
    pub resources: Option<serde_json::Value>,
    /// Prompt-related capabilities.
    pub prompts: Option<serde_json::Value>,
}

/// Describes a single tool exposed by an MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolInfo {
    /// Unique name of the tool.
    pub name: String,
    /// Optional human-readable description.
    pub description: Option<String>,
    /// JSON Schema describing the accepted input.
    pub input_schema: serde_json::Value,
}

/// Describes a single resource exposed by an MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResourceInfo {
    /// Unique URI of the resource.
    pub uri: String,
    /// Human-readable name.
    pub name: String,
    /// Optional description.
    pub description: Option<String>,
    /// MIME type, if known.
    pub mime_type: Option<String>,
}

// CROSS-DEP: dev1 — placeholder for McpServerConfig until claude_core is ready.
/// Temporary stand-in for `claude_core::McpServerConfig`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Executable command.
    pub command: String,
    /// Command-line arguments.
    #[serde(default)]
    pub args: Vec<String>,
    /// Extra environment variables.
    #[serde(default)]
    pub env: std::collections::HashMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_serialization_basic() {
        let req = JsonRpcRequest::new(1, "initialize", None);
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"method\":\"initialize\""));
        assert!(json.contains("\"id\":1"));
        // params should be absent
        assert!(!json.contains("\"params\""));
    }

    #[test]
    fn request_serialization_with_params() {
        let params = serde_json::json!({"name": "my_tool"});
        let req = JsonRpcRequest::new(42, "tools/call", Some(params));
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"id\":42"));
        assert!(json.contains("\"params\""));
        assert!(json.contains("my_tool"));
    }

    #[test]
    fn notification_serialization() {
        let notif = JsonRpcNotification::new("notifications/initialized", None);
        let json = serde_json::to_string(&notif).unwrap();
        assert!(json.contains("\"method\":\"notifications/initialized\""));
        assert!(!json.contains("\"id\""));
    }

    #[test]
    fn response_deserialization_success() {
        let json = r#"{"jsonrpc":"2.0","id":1,"result":{"capabilities":{}}}"#;
        let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.id, Some(1));
        assert!(resp.result.is_some());
        assert!(resp.error.is_none());
    }

    #[test]
    fn response_deserialization_error() {
        let json = r#"{"jsonrpc":"2.0","id":2,"error":{"code":-32601,"message":"Method not found"}}"#;
        let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.id, Some(2));
        assert!(resp.result.is_none());
        let err = resp.error.unwrap();
        assert_eq!(err.code, -32601);
        assert_eq!(err.message, "Method not found");
    }

    #[test]
    fn response_deserialization_notification_style() {
        let json = r#"{"jsonrpc":"2.0","id":null,"result":null}"#;
        let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();
        assert!(resp.id.is_none());
    }

    #[test]
    fn tool_info_roundtrip() {
        let tool = McpToolInfo {
            name: "bash".to_owned(),
            description: Some("Execute shell commands".to_owned()),
            input_schema: serde_json::json!({"type": "object", "properties": {"command": {"type": "string"}}}),
        };
        let json = serde_json::to_string(&tool).unwrap();
        let parsed: McpToolInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "bash");
        assert_eq!(parsed.description.as_deref(), Some("Execute shell commands"));
    }

    #[test]
    fn tool_info_no_description() {
        let json = r#"{"name":"my_tool","input_schema":{"type":"object"}}"#;
        let tool: McpToolInfo = serde_json::from_str(json).unwrap();
        assert_eq!(tool.name, "my_tool");
        assert!(tool.description.is_none());
        assert_eq!(tool.input_schema["type"], "object");
    }

    #[test]
    fn resource_info_roundtrip() {
        let res = McpResourceInfo {
            uri: "file:///data.json".to_owned(),
            name: "data".to_owned(),
            description: Some("A data file".to_owned()),
            mime_type: Some("application/json".to_owned()),
        };
        let json = serde_json::to_string(&res).unwrap();
        let parsed: McpResourceInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.uri, "file:///data.json");
        assert_eq!(parsed.name, "data");
        assert_eq!(parsed.description.as_deref(), Some("A data file"));
        assert_eq!(parsed.mime_type.as_deref(), Some("application/json"));
    }

    #[test]
    fn resource_info_optional_fields() {
        let json = r#"{"uri":"x://y","name":"test"}"#;
        let res: McpResourceInfo = serde_json::from_str(json).unwrap();
        assert_eq!(res.uri, "x://y");
        assert_eq!(res.name, "test");
        assert!(res.description.is_none());
        assert!(res.mime_type.is_none());
    }

    #[test]
    fn server_config_serde_roundtrip() {
        let cfg = McpServerConfig {
            command: "npx".to_owned(),
            args: vec!["mcp-server".to_owned(), "--port".to_owned(), "3000".to_owned()],
            env: {
                let mut m = std::collections::HashMap::new();
                m.insert("NODE_ENV".to_owned(), "production".to_owned());
                m
            },
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let parsed: McpServerConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.command, "npx");
        assert_eq!(parsed.args.len(), 3);
        assert_eq!(parsed.env.get("NODE_ENV").unwrap(), "production");
    }

    #[test]
    fn server_config_defaults_empty() {
        let json = r#"{"command":"echo"}"#;
        let cfg: McpServerConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.command, "echo");
        assert!(cfg.args.is_empty());
        assert!(cfg.env.is_empty());
    }

    #[test]
    fn response_with_error_data() {
        let json = r#"{"jsonrpc":"2.0","id":3,"error":{"code":-32602,"message":"Invalid params","data":{"details":"missing field"}}}"#;
        let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();
        let err = resp.error.unwrap();
        assert_eq!(err.code, -32602);
        assert!(err.data.is_some());
        assert_eq!(err.data.unwrap()["details"], "missing field");
    }
}
