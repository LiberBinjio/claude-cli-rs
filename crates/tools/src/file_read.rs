//! `FileReadTool` — read file contents with optional line-range filtering.

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value;

use claude_core::tool::{PermissionCheck, Tool, ToolInputSchema, ToolResult, ToolUseContext};
use claude_utils::fs;

/// Maximum lines returned for a full-file read (no range specified).
const MAX_LINES: usize = 2000;

/// Reads file contents, optionally restricted to a 1-indexed line range.
pub struct FileReadTool;

#[derive(Debug, Deserialize)]
struct FileReadInput {
    path: String,
    #[serde(default)]
    start_line: Option<usize>,
    #[serde(default)]
    end_line: Option<usize>,
}

#[async_trait]
impl Tool for FileReadTool {
    #[inline]
    fn name(&self) -> &str {
        "FileRead"
    }

    fn description(&self) -> &str {
        include_str!("prompts/file_read.txt")
    }

    fn input_schema(&self) -> ToolInputSchema {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Absolute or relative file path" },
                "start_line": { "type": "integer", "description": "1-indexed start line (inclusive)" },
                "end_line": { "type": "integer", "description": "1-indexed end line (inclusive)" }
            },
            "required": ["path"]
        })
    }

    #[inline]
    fn is_read_only(&self, _input: &Value) -> bool {
        true
    }

    #[inline]
    fn needs_permission(&self, _input: &Value) -> PermissionCheck {
        PermissionCheck::Allowed
    }

    async fn call(&self, input: Value, ctx: &mut ToolUseContext) -> anyhow::Result<ToolResult> {
        let params: FileReadInput = serde_json::from_value(input)?;
        let path = fs::resolve_path(&ctx.cwd, &params.path);

        if !path.exists() {
            return Ok(ToolResult::error(format!(
                "File not found: {}",
                path.display()
            )));
        }

        // Binary check
        if fs::is_binary_file(&path)? {
            let meta = std::fs::metadata(&path)?;
            return Ok(ToolResult::text(format!(
                "[Binary file, {} bytes]",
                meta.len()
            )));
        }

        let start = params.start_line.unwrap_or(1).max(1);
        let end = params
            .end_line
            .unwrap_or(start + MAX_LINES - 1)
            .max(start);

        let content = fs::read_file_in_range(&path, start, end)?;

        // Add line-number prefixes
        let width = end.to_string().len();
        let mut result = String::with_capacity(content.len() + content.lines().count() * (width + 4));
        for (i, line) in content.lines().enumerate() {
            use std::fmt::Write;
            let _ = writeln!(result, "{:>width$} │ {line}", start + i, width = width);
        }

        Ok(ToolResult::text(result))
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
    async fn read_full_file() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "line1\nline2\nline3\n").unwrap();

        let tool = FileReadTool;
        let result = tool
            .call(
                serde_json::json!({ "path": file.to_str().unwrap() }),
                &mut ctx(&dir),
            )
            .await
            .unwrap();
        assert!(!result.is_error);
        let text = result.content[0].text.as_deref().unwrap();
        assert!(text.contains("line1"));
        assert!(text.contains("line3"));
    }

    #[tokio::test]
    async fn read_line_range() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "a\nb\nc\nd\ne\n").unwrap();

        let tool = FileReadTool;
        let result = tool
            .call(
                serde_json::json!({ "path": file.to_str().unwrap(), "start_line": 2, "end_line": 3 }),
                &mut ctx(&dir),
            )
            .await
            .unwrap();
        let text = result.content[0].text.as_deref().unwrap();
        assert!(text.contains("b"));
        assert!(text.contains("c"));
        assert!(!text.contains("│ a"));
    }

    #[tokio::test]
    async fn binary_file_detection() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("binary.bin");
        std::fs::write(&file, b"\x00\x01\x02binary").unwrap();

        let tool = FileReadTool;
        let result = tool
            .call(
                serde_json::json!({ "path": file.to_str().unwrap() }),
                &mut ctx(&dir),
            )
            .await
            .unwrap();
        let text = result.content[0].text.as_deref().unwrap();
        assert!(text.contains("[Binary file"));
    }

    #[tokio::test]
    async fn file_not_found() {
        let dir = TempDir::new().unwrap();
        let tool = FileReadTool;
        let result = tool
            .call(
                serde_json::json!({ "path": "/nonexistent/file.txt" }),
                &mut ctx(&dir),
            )
            .await
            .unwrap();
        assert!(result.is_error);
        let text = result.content[0].text.as_deref().unwrap();
        assert!(text.contains("not found"));
    }

    #[test]
    fn is_read_only_always() {
        let tool = FileReadTool;
        assert!(tool.is_read_only(&serde_json::json!({})));
    }

    #[test]
    fn permission_always_allowed() {
        let tool = FileReadTool;
        assert_eq!(
            tool.needs_permission(&serde_json::json!({})),
            PermissionCheck::Allowed
        );
    }

    #[tokio::test]
    async fn line_numbers_are_correct() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("nums.txt");
        std::fs::write(&file, "alpha\nbeta\ngamma\ndelta\n").unwrap();

        let tool = FileReadTool;
        let result = tool
            .call(
                serde_json::json!({ "path": file.to_str().unwrap(), "start_line": 2, "end_line": 3 }),
                &mut ctx(&dir),
            )
            .await
            .unwrap();
        let text = result.content[0].text.as_deref().unwrap();
        assert!(text.contains("2 │ beta"));
        assert!(text.contains("3 │ gamma"));
    }
}
