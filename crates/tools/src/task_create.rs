//! TaskCreateTool: create a new background task.

use claude_core::tool::{Tool, ToolInputSchema, ToolResult, ToolUseContext};
use async_trait::async_trait;
use serde_json::Value;

/// Tool for creating background tasks.
pub struct TaskCreateTool;

#[async_trait]
impl Tool for TaskCreateTool {
    fn name(&self) -> &str { "TaskCreate" }
    fn description(&self) -> &str { "Create a new background task." }

    fn input_schema(&self) -> ToolInputSchema {
        serde_json::json!({
            "type": "object",
            "properties": {
                "description": { "type": "string", "description": "Task description" },
                "prompt": { "type": "string", "description": "Prompt for the task" }
            },
            "required": ["description"]
        })
    }

    fn is_read_only(&self, _input: &Value) -> bool { false }

    async fn call(&self, input: Value, _ctx: &mut ToolUseContext) -> anyhow::Result<ToolResult> {
        let desc = input
            .get("description")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing 'description'"))?;

        let id = crate::shared::create_task(desc);
        Ok(ToolResult::text(format!("Task created with ID: {id}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_create_task() {
        let tool = TaskCreateTool;
        let mut ctx = ToolUseContext {
            cwd: PathBuf::from("."),
            permission_mode: claude_core::permission::PermissionMode::Default,
            tool_use_id: "t".into(),
            session_id: "s".into(),
        };
        let input = serde_json::json!({"description": "test task"});
        let result = tool.call(input, &mut ctx).await.unwrap();
        assert!(!result.is_error);
        let text = result.content[0].text.as_deref().unwrap_or("");
        assert!(text.contains("Task created"));
    }
}
