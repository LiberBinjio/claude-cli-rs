//! Integration tests for MCP types (claude_mcp).

use claude_mcp::types::*;

#[test]
fn json_rpc_request_basic() {
    let req = JsonRpcRequest::new(1, "initialize", None);
    let json = serde_json::to_string(&req).unwrap();
    assert!(json.contains("\"jsonrpc\":\"2.0\""));
    assert!(json.contains("\"method\":\"initialize\""));
    assert!(json.contains("\"id\":1"));
    assert!(!json.contains("\"params\""));
}

#[test]
fn json_rpc_request_with_params() {
    let params = serde_json::json!({"name": "test_tool"});
    let req = JsonRpcRequest::new(10, "tools/call", Some(params));
    let json = serde_json::to_string(&req).unwrap();
    assert!(json.contains("\"id\":10"));
    assert!(json.contains("\"params\""));
    assert!(json.contains("test_tool"));
}

#[test]
fn json_rpc_notification_no_id() {
    let notif = JsonRpcNotification::new("notifications/initialized", None);
    let json = serde_json::to_string(&notif).unwrap();
    assert!(!json.contains("\"id\""));
    assert!(json.contains("\"method\":\"notifications/initialized\""));
}

#[test]
fn json_rpc_response_success() {
    let json = r#"{"jsonrpc":"2.0","id":1,"result":{"capabilities":{}}}"#;
    let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();
    assert_eq!(resp.id, Some(1));
    assert!(resp.result.is_some());
    assert!(resp.error.is_none());
}

#[test]
fn json_rpc_response_error() {
    let json = r#"{"jsonrpc":"2.0","id":2,"error":{"code":-32601,"message":"Method not found"}}"#;
    let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();
    let err = resp.error.unwrap();
    assert_eq!(err.code, -32601);
    assert_eq!(err.message, "Method not found");
}

#[test]
fn json_rpc_response_null_id() {
    let json = r#"{"jsonrpc":"2.0","id":null,"result":null}"#;
    let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();
    assert!(resp.id.is_none());
}

#[test]
fn mcp_tool_info_roundtrip() {
    let tool = McpToolInfo {
        name: "bash".to_owned(),
        description: Some("Execute commands".to_owned()),
        input_schema: serde_json::json!({"type": "object"}),
    };
    let json = serde_json::to_string(&tool).unwrap();
    let parsed: McpToolInfo = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.name, "bash");
    assert_eq!(parsed.description.as_deref(), Some("Execute commands"));
}

#[test]
fn mcp_tool_info_no_description() {
    let json = r#"{"name":"my_tool","input_schema":{}}"#;
    let tool: McpToolInfo = serde_json::from_str(json).unwrap();
    assert!(tool.description.is_none());
}

#[test]
fn mcp_resource_info_roundtrip() {
    let res = McpResourceInfo {
        uri: "file:///data.json".to_owned(),
        name: "data".to_owned(),
        description: Some("A data file".to_owned()),
        mime_type: Some("application/json".to_owned()),
    };
    let json = serde_json::to_string(&res).unwrap();
    let parsed: McpResourceInfo = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.uri, "file:///data.json");
    assert_eq!(parsed.mime_type.as_deref(), Some("application/json"));
}

#[test]
fn mcp_resource_info_optional_fields() {
    let json = r#"{"uri":"x://y","name":"test"}"#;
    let res: McpResourceInfo = serde_json::from_str(json).unwrap();
    assert!(res.description.is_none());
    assert!(res.mime_type.is_none());
}

#[test]
fn connection_manager_empty() {
    let mgr = claude_mcp::McpConnectionManager::new();
    // new manager has no clients — no panics
    drop(mgr);
}

#[test]
fn connection_manager_get_nonexistent() {
    let mut mgr = claude_mcp::McpConnectionManager::new();
    assert!(mgr.get("nonexistent").is_none());
}
