//! TaskGetTool: query a task's status.

use claude_core::tool::{PermissionCheck, Tool, ToolInputSchema, ToolResult, ToolUseContext};
use async_trait::async_trait;
use serde_json::Value;

/// Tool for querying task status by ID.
pub struct TaskGetTool;

#[async_trait]
impl Tool for TaskGetTool {
    fn name(&self) -> &str { "TaskGet" }
    fn description(&self) -> &str { "Get the status and details of a background task." }

    fn input_schema(&self) -> ToolInputSchema {
        serde_json::json!({
            "type": "object",
            "properties": {
                "task_id": { "type": "string", "description": "Task ID to query" }
            },
            "required": ["task_id"]
        })
    }

    fn is_read_only(&self, _input: &Value) -> bool { true }

    fn needs_permission(&self, _input: &Value) -> PermissionCheck {
        PermissionCheck::Allowed
    }

    async fn call(&self, input: Value, _ctx: &mut ToolUseContext) -> anyhow::Result<ToolResult> {
        let task_id = input
            .get("task_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing 'task_id'"))?;

        let mgr = crate::shared::task_manager();
        let guard = mgr.lock().map_err(|e| anyhow::anyhow!("lock error: {e}"))?;

        match guard.get(task_id) {
            Some(entry) => {
                let json = serde_json::to_string_pretty(entry)?;
                Ok(ToolResult::text(json))
            }
            None => Ok(ToolResult::error(format!("Task not found: {task_id}"))),
        }
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
            tool_use_id: "t".into(),
            session_id: "s".into(),
        }
    }

    #[tokio::test]
    async fn test_get_existing() {
        let id = crate::shared::create_task("lookup test");
        let tool = TaskGetTool;
        let mut ctx = test_ctx();
        let input = serde_json::json!({"task_id": id});
        let result = tool.call(input, &mut ctx).await.unwrap();
        assert!(!result.is_error);
    }

    #[tokio::test]
    async fn test_get_missing() {
        let tool = TaskGetTool;
        let mut ctx = test_ctx();
        let input = serde_json::json!({"task_id": "nonexistent"});
        let result = tool.call(input, &mut ctx).await.unwrap();
        assert!(result.is_error);
    }
}
