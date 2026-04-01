//! `FileEditTool` — exact string replacement within a file.

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value;

use claude_core::tool::{PermissionCheck, Tool, ToolInputSchema, ToolResult, ToolUseContext};
use claude_utils::{diff, fs};

/// Performs exact string replacement in a single file, verifying uniqueness.
pub struct FileEditTool;

#[derive(Debug, Deserialize)]
struct FileEditInput {
    path: String,
    old_string: String,
    new_string: String,
}

#[async_trait]
impl Tool for FileEditTool {
    #[inline]
    fn name(&self) -> &str {
        "FileEdit"
    }

    fn description(&self) -> &str {
        include_str!("prompts/file_edit.txt")
    }

    fn input_schema(&self) -> ToolInputSchema {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "File path to edit" },
                "old_string": { "type": "string", "description": "Exact text to replace (must appear exactly once)" },
                "new_string": { "type": "string", "description": "Replacement text" }
            },
            "required": ["path", "old_string", "new_string"]
        })
    }

    #[inline]
    fn is_read_only(&self, _input: &Value) -> bool {
        false
    }

    #[inline]
    fn needs_permission(&self, _input: &Value) -> PermissionCheck {
        PermissionCheck::NeedsAsk
    }

    async fn call(&self, input: Value, ctx: &mut ToolUseContext) -> anyhow::Result<ToolResult> {
        let params: FileEditInput = serde_json::from_value(input)?;
        let path = fs::resolve_path(&ctx.cwd, &params.path);

        let original = std::fs::read_to_string(&path)?;

        let new_content = match diff::apply_edit(&original, &params.old_string, &params.new_string)
        {
            Ok(content) => content,
            Err(e) => return Ok(ToolResult::error(e.to_string())),
        };

        // Generate unified diff for display
        let diff_output =
            diff::unified_diff(&original, &new_content, &path.display().to_string());

        fs::atomic_write(&path, &new_content)?;

        Ok(ToolResult::text(diff_output))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn ctx(dir: &TempDir) -> ToolUseContext {
        ToolUseContext {
            cwd: dir.path().to_path_buf(),
            permission_mode: claude_core::permission::PermissionMode::Default,
            tool_use_id: "test".into(),
            session_id: "s".into(),
        }
    }

    #[tokio::test]
    async fn exact_replacement() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "hello world").unwrap();

        let tool = FileEditTool;
        let result = tool
            .call(
                serde_json::json!({
                    "path": file.to_str().unwrap(),
                    "old_string": "world",
                    "new_string": "rust"
                }),
                &mut ctx(&dir),
            )
            .await
            .unwrap();
        assert!(!result.is_error);
        let content = std::fs::read_to_string(&file).unwrap();
        assert_eq!(content, "hello rust");
    }

    #[tokio::test]
    async fn not_found_errors() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "hello world").unwrap();

        let tool = FileEditTool;
        let result = tool
            .call(
                serde_json::json!({
                    "path": file.to_str().unwrap(),
                    "old_string": "nonexistent",
                    "new_string": "x"
                }),
                &mut ctx(&dir),
            )
            .await
            .unwrap();
        assert!(result.is_error);
    }

    #[tokio::test]
    async fn ambiguous_match_errors() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "aaa bbb aaa").unwrap();

        let tool = FileEditTool;
        let result = tool
            .call(
                serde_json::json!({
                    "path": file.to_str().unwrap(),
                    "old_string": "aaa",
                    "new_string": "ccc"
                }),
                &mut ctx(&dir),
            )
            .await
            .unwrap();
        assert!(result.is_error);
        let text = result.content[0].text.as_deref().unwrap();
        assert!(text.contains("ambiguous"));
    }

    #[tokio::test]
    async fn generates_unified_diff() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("diff.txt");
        std::fs::write(&file, "line1\nold_value\nline3\n").unwrap();

        let tool = FileEditTool;
        let result = tool
            .call(
                serde_json::json!({
                    "path": file.to_str().unwrap(),
                    "old_string": "old_value",
                    "new_string": "new_value"
                }),
                &mut ctx(&dir),
            )
            .await
            .unwrap();
        assert!(!result.is_error);
        let text = result.content[0].text.as_deref().unwrap();
        assert!(text.contains("-old_value"));
        assert!(text.contains("+new_value"));
    }

    #[test]
    fn needs_permission_ask() {
        let tool = FileEditTool;
        assert_eq!(
            tool.needs_permission(&serde_json::json!({})),
            PermissionCheck::NeedsAsk
        );
    }

    #[tokio::test]
    async fn nonexistent_file() {
        let dir = TempDir::new().unwrap();
        let tool = FileEditTool;
        let result = tool
            .call(
                serde_json::json!({
                    "path": "nofile.txt",
                    "old_string": "a",
                    "new_string": "b"
                }),
                &mut ctx(&dir),
            )
            .await;
        assert!(result.is_err());
    }
}
