//! McpProxyTool: proxy tool calls to an MCP server.

use claude_core::tool::{PermissionCheck, Tool, ToolInputSchema, ToolResult, ToolUseContext};
use async_trait::async_trait;
use serde_json::Value;

/// A dynamically-created tool that proxies calls to an MCP server.
pub struct McpProxyTool {
    /// Name of the MCP server.
    pub server_name: String,
    /// Tool name as reported by the MCP server.
    pub tool_name: String,
    /// Tool description from MCP.
    pub tool_description: String,
    /// JSON Schema for the tool's input.
    pub tool_schema: Value,
}

#[async_trait]
impl Tool for McpProxyTool {
    fn name(&self) -> &str { &self.tool_name }

    fn description(&self) -> &str { &self.tool_description }

    fn input_schema(&self) -> ToolInputSchema { self.tool_schema.clone() }

    fn is_read_only(&self, _input: &Value) -> bool { false }

    fn needs_permission(&self, _input: &Value) -> PermissionCheck {
        PermissionCheck::NeedsAsk
    }

    async fn call(&self, input: Value, _ctx: &mut ToolUseContext) -> anyhow::Result<ToolResult> {
        // CROSS-DEP: requires dev5's McpClient/McpConnectionManager for real calls
        Ok(ToolResult::text(format!(
            "[MCP:{}] Tool '{}' called with: {}",
            self.server_name,
            self.tool_name,
            serde_json::to_string_pretty(&input)?
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_ctx() -> ToolUseContext {
        ToolUseContext {
            cwd: PathBuf::from("."),
            permission_mode: claude_core::permission::PermissionMode::Default,
            tool_use_id: "test".into(),
            session_id: "test".into(),
        }
    }

    fn make_proxy() -> McpProxyTool {
        McpProxyTool {
            server_name: "test-server".into(),
            tool_name: "get_weather".into(),
            tool_description: "Get weather info".into(),
            tool_schema: serde_json::json!({"type": "object", "properties": {}}),
        }
    }

    #[test]
    fn test_proxy_metadata() {
        let tool = make_proxy();
        assert_eq!(tool.name(), "get_weather");
        assert_eq!(tool.description(), "Get weather info");
    }

    #[tokio::test]
    async fn test_proxy_call() {
        let tool = make_proxy();
        let mut ctx = test_ctx();
        let input = serde_json::json!({"city": "Tokyo"});
        let result = tool.call(input, &mut ctx).await.unwrap();
        let text = result.content[0].text.as_deref().unwrap_or("");
        assert!(text.contains("test-server"));
        assert!(text.contains("Tokyo"));
    }
}
