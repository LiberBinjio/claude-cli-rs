//! TaskStopTool: cancel a running task.

use claude_core::tool::{Tool, ToolInputSchema, ToolResult, ToolUseContext};
use async_trait::async_trait;
use serde_json::Value;

/// Tool for stopping/cancelling a background task.
pub struct TaskStopTool;

#[async_trait]
impl Tool for TaskStopTool {
    fn name(&self) -> &str { "TaskStop" }
    fn description(&self) -> &str { "Stop a running background task." }

    fn input_schema(&self) -> ToolInputSchema {
        serde_json::json!({
            "type": "object",
            "properties": {
                "task_id": { "type": "string", "description": "Task ID to stop" }
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

        entry.status = crate::shared::TaskStatus::Cancelled;
        Ok(ToolResult::text(format!("Task {task_id} cancelled")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_stop_task() {
        let id = crate::shared::create_task("stop me");
        let tool = TaskStopTool;
        let mut ctx = ToolUseContext {
            cwd: PathBuf::from("."),
            permission_mode: claude_core::permission::PermissionMode::Default,
            tool_use_id: "t".into(),
            session_id: "s".into(),
        };
        let input = serde_json::json!({"task_id": id});
        let result = tool.call(input, &mut ctx).await.unwrap();
        assert!(!result.is_error);
        let text = result.content[0].text.as_deref().unwrap_or("");
        assert!(text.contains("cancelled"));
    }
}
