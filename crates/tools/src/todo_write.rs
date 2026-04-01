//! TodoWriteTool: manage a todo list in ~/.claude/todos.json.

use claude_core::tool::{Tool, ToolInputSchema, ToolResult, ToolUseContext};
use async_trait::async_trait;
use serde_json::Value;

/// Tool for reading and writing the todo list.
pub struct TodoWriteTool;

#[async_trait]
impl Tool for TodoWriteTool {
    fn name(&self) -> &str { "TodoWrite" }

    fn description(&self) -> &str {
        "Create or update the todo list stored in ~/.claude/todos.json."
    }

    fn input_schema(&self) -> ToolInputSchema {
        serde_json::json!({
            "type": "object",
            "properties": {
                "todos": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "id": { "type": "string" },
                            "title": { "type": "string" },
                            "status": { "type": "string", "enum": ["not-started", "in-progress", "completed"] }
                        },
                        "required": ["id", "title", "status"]
                    },
                    "description": "Complete list of todos (replaces existing)"
                }
            },
            "required": ["todos"]
        })
    }

    fn is_read_only(&self, _input: &Value) -> bool { false }

    async fn call(&self, input: Value, _ctx: &mut ToolUseContext) -> anyhow::Result<ToolResult> {
        let todos = input
            .get("todos")
            .ok_or_else(|| anyhow::anyhow!("missing 'todos' parameter"))?;

        let Some(dir) = crate::shared::claude_home_dir() else {
            return Ok(ToolResult::error("Cannot determine home directory"));
        };
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("todos.json");
        let json = serde_json::to_string_pretty(todos)?;
        std::fs::write(&path, &json)?;

        let count = todos.as_array().map(|a| a.len()).unwrap_or(0);
        Ok(ToolResult::text(format!("Saved {count} todos to {}", path.display())))
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
        let tool = TodoWriteTool;
        assert_eq!(tool.name(), "TodoWrite");
        assert!(!tool.is_read_only(&serde_json::json!({})));
    }

    #[tokio::test]
    async fn test_write_todos() {
        let tool = TodoWriteTool;
        let mut ctx = test_ctx();
        let input = serde_json::json!({
            "todos": [
                {"id": "1", "title": "Test", "status": "not-started"}
            ]
        });
        let result = tool.call(input, &mut ctx).await.unwrap();
        let text = result.content[0].text.as_deref().unwrap_or("");
        assert!(text.contains("1 todos"));
    }
}
