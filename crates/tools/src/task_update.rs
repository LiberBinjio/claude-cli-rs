//! TaskUpdateTool: update task description or status.

use claude_core::tool::{Tool, ToolInputSchema, ToolResult, ToolUseContext};
use async_trait::async_trait;
use serde_json::Value;

/// Tool for updating an existing task.
pub struct TaskUpdateTool;

#[async_trait]
impl Tool for TaskUpdateTool {
    fn name(&self) -> &str { "TaskUpdate" }
    fn description(&self) -> &str { "Update a task's description or status." }

    fn input_schema(&self) -> ToolInputSchema {
        serde_json::json!({
            "type": "object",
            "properties": {
                "task_id": { "type": "string", "description": "Task ID" },
                "description": { "type": "string", "description": "New description" },
                "status": { "type": "string", "description": "New status" }
            },
            "required": ["task_id"]
        })
    }

    fn is_read_only(&self, _input: &Value) -> bool { false }

    async fn call(&self, input: Value, _ctx: &mut ToolUseContext) -> anyhow::Result<ToolResult> {
        let task_id = input
            .get("task_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing 'task_id'"))?;

        let mgr = crate::shared::task_manager();
        let mut guard = mgr.lock().map_err(|e| anyhow::anyhow!("lock error: {e}"))?;

        let Some(entry) = guard.get_mut(task_id) else {
            return Ok(ToolResult::error(format!("Task not found: {task_id}")));
        };

        if let Some(desc) = input.get("description").and_then(|v| v.as_str()) {
            entry.description = desc.to_string();
        }
        if let Some(status) = input.get("status").and_then(|v| v.as_str()) {
            entry.status = match status {
                "pending" => crate::shared::TaskStatus::Pending,
                "running" => crate::shared::TaskStatus::Running,
                "completed" => crate::shared::TaskStatus::Completed,
                "failed" => crate::shared::TaskStatus::Failed,
                "cancelled" => crate::shared::TaskStatus::Cancelled,
                other => return Ok(ToolResult::error(format!("Invalid status: {other}"))),
            };
        }

        Ok(ToolResult::text(format!("Task {task_id} updated")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_update_task() {
        let id = crate::shared::create_task("update me");
        let tool = TaskUpdateTool;
        let mut ctx = ToolUseContext {
            cwd: PathBuf::from("."),
            permission_mode: claude_core::permission::PermissionMode::Default,
            tool_use_id: "t".into(),
            session_id: "s".into(),
        };
        let input = serde_json::json!({
            "task_id": id,
            "description": "updated desc",
            "status": "running"
        });
        let result = tool.call(input, &mut ctx).await.unwrap();
        assert!(!result.is_error);
    }
}
