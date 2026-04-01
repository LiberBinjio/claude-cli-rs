//! AgentTool: launch a sub-agent to handle delegated tasks.

use claude_core::tool::{PermissionCheck, Tool, ToolInputSchema, ToolResult, ToolUseContext};
use async_trait::async_trait;
use serde_json::Value;

/// Tool for delegating tasks to a sub-agent.
pub struct AgentTool;

#[async_trait]
impl Tool for AgentTool {
    fn name(&self) -> &str { "Agent" }

    fn description(&self) -> &str {
        "Launch a sub-agent to handle a delegated task. The sub-agent has \
         access to the same tools and can work independently."
    }

    fn input_schema(&self) -> ToolInputSchema {
        serde_json::json!({
            "type": "object",
            "properties": {
                "task": {
                    "type": "string",
                    "description": "Description of the task to delegate"
                },
                "tools": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Optional: restrict which tools the sub-agent can use"
                }
            },
            "required": ["task"]
        })
    }

    fn is_read_only(&self, _input: &Value) -> bool { false }

    fn needs_permission(&self, _input: &Value) -> PermissionCheck {
        PermissionCheck::NeedsAsk
    }

    async fn call(&self, input: Value, _ctx: &mut ToolUseContext) -> anyhow::Result<ToolResult> {
        let task = input
            .get("task")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing 'task' parameter"))?;

        // TODO(T17): Full integration with QueryEngine for real sub-agent loops
        Ok(ToolResult::text(format!(
            "[Sub-agent] Task received: {task}\n\
             Note: Full sub-agent execution requires QueryEngine integration (T17).\n\
             The task has been acknowledged."
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

    #[test]
    fn test_schema() {
        let tool = AgentTool;
        assert_eq!(tool.name(), "Agent");
        assert_eq!(tool.input_schema()["required"][0], "task");
    }

    #[test]
    fn test_not_read_only() {
        let tool = AgentTool;
        assert!(!tool.is_read_only(&serde_json::json!({})));
    }

    #[tokio::test]
    async fn test_agent_placeholder() {
        let tool = AgentTool;
        let mut ctx = test_ctx();
        let input = serde_json::json!({"task": "write a test"});
        let result = tool.call(input, &mut ctx).await.unwrap();
        assert!(!result.is_error);
        let text = result.content[0].text.as_deref().unwrap_or("");
        assert!(text.contains("write a test"));
    }
}
