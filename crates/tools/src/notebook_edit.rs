//! NotebookEditTool: edit Jupyter notebook (.ipynb) cells.

use claude_core::tool::{Tool, ToolInputSchema, ToolResult, ToolUseContext};
use async_trait::async_trait;
use serde_json::Value;

/// Tool for editing cells in Jupyter notebooks.
pub struct NotebookEditTool;

#[async_trait]
impl Tool for NotebookEditTool {
    fn name(&self) -> &str { "NotebookEdit" }
    fn description(&self) -> &str { "Edit a cell in a Jupyter notebook (.ipynb file)." }

    fn input_schema(&self) -> ToolInputSchema {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Path to the .ipynb file" },
                "cell_index": { "type": "integer", "description": "0-based cell index to edit" },
                "new_source": { "type": "string", "description": "New source content for the cell" }
            },
            "required": ["path", "cell_index", "new_source"]
        })
    }

    fn is_read_only(&self, _input: &Value) -> bool { false }

    async fn call(&self, input: Value, ctx: &mut ToolUseContext) -> anyhow::Result<ToolResult> {
        let path_str = input
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing 'path'"))?;
        let cell_index = input
            .get("cell_index")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow::anyhow!("missing 'cell_index'"))? as usize;
        let new_source = input
            .get("new_source")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing 'new_source'"))?;

        let full_path = ctx.cwd.join(path_str);

        // Read notebook JSON
        let content = std::fs::read_to_string(&full_path)
            .map_err(|e| anyhow::anyhow!("Cannot read {}: {e}", full_path.display()))?;
        let mut notebook: Value = serde_json::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Invalid notebook JSON: {e}"))?;

        // Navigate to cells array
        let cells = notebook
            .get_mut("cells")
            .and_then(|c| c.as_array_mut())
            .ok_or_else(|| anyhow::anyhow!("No 'cells' array in notebook"))?;

        if cell_index >= cells.len() {
            return Ok(ToolResult::error(format!(
                "Cell index {cell_index} out of range (notebook has {} cells)",
                cells.len()
            )));
        }

        // Update cell source (as array of lines)
        let source_lines: Vec<Value> = new_source
            .lines()
            .map(|l| Value::String(format!("{l}\n")))
            .collect();
        cells[cell_index]["source"] = Value::Array(source_lines);

        // Write back
        let json = serde_json::to_string_pretty(&notebook)?;
        std::fs::write(&full_path, json)?;

        Ok(ToolResult::text(format!(
            "Updated cell {cell_index} in {}",
            path_str
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema() {
        let tool = NotebookEditTool;
        assert_eq!(tool.name(), "NotebookEdit");
        assert!(!tool.is_read_only(&serde_json::json!({})));
    }

    #[tokio::test]
    async fn test_edit_notebook() {
        let dir = tempfile::tempdir().unwrap();
        let nb_path = dir.path().join("test.ipynb");
        let notebook = serde_json::json!({
            "cells": [
                {"cell_type": "code", "source": ["print('hello')\n"], "metadata": {}},
                {"cell_type": "code", "source": ["x = 1\n"], "metadata": {}}
            ],
            "metadata": {},
            "nbformat": 4,
            "nbformat_minor": 5
        });
        std::fs::write(&nb_path, serde_json::to_string(&notebook).unwrap()).unwrap();

        let tool = NotebookEditTool;
        let mut ctx = ToolUseContext {
            cwd: dir.path().to_path_buf(),
            permission_mode: claude_core::permission::PermissionMode::Default,
            tool_use_id: "t".into(),
            session_id: "s".into(),
        };
        let input = serde_json::json!({
            "path": "test.ipynb",
            "cell_index": 0,
            "new_source": "print('updated')"
        });
        let result = tool.call(input, &mut ctx).await.unwrap();
        assert!(!result.is_error);

        // Verify
        let updated: Value = serde_json::from_str(&std::fs::read_to_string(&nb_path).unwrap()).unwrap();
        let src = updated["cells"][0]["source"][0].as_str().unwrap();
        assert!(src.contains("updated"));
    }
}
