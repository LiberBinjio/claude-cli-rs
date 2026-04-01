//! TaskOutputTool: get the output of a background task.

use claude_core::tool::{PermissionCheck, Tool, ToolInputSchema, ToolResult, ToolUseContext};
use async_trait::async_trait;
use serde_json::Value;

/// Tool for retrieving a task's accumulated output.
pub struct TaskOutputTool;

#[async_trait]
impl Tool for TaskOutputTool {
    fn name(&self) -> &str { "TaskOutput" }
    fn description(&self) -> &str { "Get the output of a background task." }

    fn input_schema(&self) -> ToolInputSchema {
        serde_json::json!({
            "type": "object",
            "properties": {
                "task_id": { "type": "string", "description": "Task ID" }
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
            Some(entry) if entry.output.is_empty() => {
                Ok(ToolResult::text(format!(
                    "Task {task_id} ({}) has no output yet.",
                    entry.status
                )))
            }
            Some(entry) => Ok(ToolResult::text(entry.output.clone())),
            None => Ok(ToolResult::error(format!("Task not found: {task_id}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_get_output() {
        let id = crate::shared::create_task("output test");
        let tool = TaskOutputTool;
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
        assert!(text.contains("no output"));
    }
}
