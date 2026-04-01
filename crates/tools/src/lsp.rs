//! LspTool: execute LSP commands (placeholder).

use claude_core::tool::{PermissionCheck, Tool, ToolInputSchema, ToolResult, ToolUseContext};
use async_trait::async_trait;
use serde_json::Value;

/// Tool for interacting with Language Server Protocol servers.
pub struct LspTool;

#[async_trait]
impl Tool for LspTool {
    fn name(&self) -> &str { "LSP" }
    fn description(&self) -> &str { "Execute LSP commands for code intelligence." }

    fn input_schema(&self) -> ToolInputSchema {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": { "type": "string", "description": "LSP command to execute" },
                "args": { "type": "object", "description": "Command arguments" }
            },
            "required": ["command"]
        })
    }

    fn is_read_only(&self, _input: &Value) -> bool { true }

    fn needs_permission(&self, _input: &Value) -> PermissionCheck {
        PermissionCheck::Allowed
    }

    async fn call(&self, _input: Value, _ctx: &mut ToolUseContext) -> anyhow::Result<ToolResult> {
        // TODO: Integrate with IDE bridge for real LSP access
        Ok(ToolResult::error("Not yet implemented: LSP"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_schema() {
        let tool = LspTool;
        assert_eq!(tool.name(), "LSP");
        assert!(tool.is_read_only(&serde_json::json!({})));
    }

    #[tokio::test]
    async fn test_placeholder() {
        let tool = LspTool;
        let mut ctx = ToolUseContext {
            cwd: PathBuf::from("."),
            permission_mode: claude_core::permission::PermissionMode::Default,
            tool_use_id: "t".into(),
            session_id: "s".into(),
        };
        let result = tool.call(serde_json::json!({"command": "hover"}), &mut ctx).await.unwrap();
        assert!(result.is_error);
    }
}
