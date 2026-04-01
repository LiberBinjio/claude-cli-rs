//! TaskListTool: list all background tasks.

use claude_core::tool::{PermissionCheck, Tool, ToolInputSchema, ToolResult, ToolUseContext};
use async_trait::async_trait;
use serde_json::Value;

/// Tool for listing all tracked tasks.
pub struct TaskListTool;

#[async_trait]
impl Tool for TaskListTool {
    fn name(&self) -> &str { "TaskList" }
    fn description(&self) -> &str { "List all background tasks." }

    fn input_schema(&self) -> ToolInputSchema {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    fn is_read_only(&self, _input: &Value) -> bool { true }

    fn needs_permission(&self, _input: &Value) -> PermissionCheck {
        PermissionCheck::Allowed
    }

    async fn call(&self, _input: Value, _ctx: &mut ToolUseContext) -> anyhow::Result<ToolResult> {
        let mgr = crate::shared::task_manager();
        let guard = mgr.lock().map_err(|e| anyhow::anyhow!("lock error: {e}"))?;

        if guard.is_empty() {
            return Ok(ToolResult::text("No tasks."));
        }

        let mut tasks: Vec<_> = guard.values().cloned().collect();
        tasks.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        let json = serde_json::to_string_pretty(&tasks)?;
        Ok(ToolResult::text(json))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_list_tasks() {
        let tool = TaskListTool;
        let mut ctx = ToolUseContext {
            cwd: PathBuf::from("."),
            permission_mode: claude_core::permission::PermissionMode::Default,
            tool_use_id: "t".into(),
            session_id: "s".into(),
        };
        let result = tool.call(serde_json::json!({}), &mut ctx).await.unwrap();
        assert!(!result.is_error);
    }
}
