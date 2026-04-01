//! TeamCreateTool: create an agent team (placeholder).

use claude_core::tool::{Tool, ToolInputSchema, ToolResult, ToolUseContext};
use async_trait::async_trait;
use serde_json::Value;

/// Tool for creating agent teams.
pub struct TeamCreateTool;

#[async_trait]
impl Tool for TeamCreateTool {
    fn name(&self) -> &str { "TeamCreate" }
    fn description(&self) -> &str { "Create a new agent team." }

    fn input_schema(&self) -> ToolInputSchema {
        serde_json::json!({
            "type": "object",
            "properties": {
                "team_name": { "type": "string", "description": "Name for the team" },
                "agent_count": { "type": "integer", "description": "Number of agents" }
            },
            "required": ["team_name"]
        })
    }

    fn is_read_only(&self, _input: &Value) -> bool { false }

    async fn call(&self, _input: Value, _ctx: &mut ToolUseContext) -> anyhow::Result<ToolResult> {
        Ok(ToolResult::error("Not yet implemented: TeamCreate"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_placeholder() {
        let tool = TeamCreateTool;
        let mut ctx = ToolUseContext {
            cwd: PathBuf::from("."),
            permission_mode: claude_core::permission::PermissionMode::Default,
            tool_use_id: "t".into(),
            session_id: "s".into(),
        };
        let result = tool.call(serde_json::json!({"team_name": "x"}), &mut ctx).await.unwrap();
        assert!(result.is_error);
    }
}
