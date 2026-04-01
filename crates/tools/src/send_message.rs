//! SendMessageTool: send a message to a team agent (placeholder).

use claude_core::tool::{Tool, ToolInputSchema, ToolResult, ToolUseContext};
use async_trait::async_trait;
use serde_json::Value;

/// Tool for sending messages to agents within a team.
pub struct SendMessageTool;

#[async_trait]
impl Tool for SendMessageTool {
    fn name(&self) -> &str { "SendMessage" }
    fn description(&self) -> &str { "Send a message to an agent in a team." }

    fn input_schema(&self) -> ToolInputSchema {
        serde_json::json!({
            "type": "object",
            "properties": {
                "agent_id": { "type": "string", "description": "Target agent ID" },
                "message": { "type": "string", "description": "Message content" }
            },
            "required": ["agent_id", "message"]
        })
    }

    fn is_read_only(&self, _input: &Value) -> bool { false }

    async fn call(&self, _input: Value, _ctx: &mut ToolUseContext) -> anyhow::Result<ToolResult> {
        Ok(ToolResult::error("Not yet implemented: SendMessage"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_placeholder() {
        let tool = SendMessageTool;
        let mut ctx = ToolUseContext {
            cwd: PathBuf::from("."),
            permission_mode: claude_core::permission::PermissionMode::Default,
            tool_use_id: "t".into(),
            session_id: "s".into(),
        };
        let input = serde_json::json!({"agent_id": "a1", "message": "hi"});
        let result = tool.call(input, &mut ctx).await.unwrap();
        assert!(result.is_error);
    }
}
