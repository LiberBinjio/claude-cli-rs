//! `FileWriteTool` — create or overwrite files with atomic writes.

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value;

use claude_core::tool::{PermissionCheck, Tool, ToolInputSchema, ToolResult, ToolUseContext};
use claude_utils::fs;

/// Filename prefixes that indicate sensitive/hidden files.
const SENSITIVE_PREFIXES: &[&str] = &[".env", ".ssh", ".gnupg", ".aws", ".npmrc", ".netrc"];

/// Atomically writes content to a file, creating parent directories as needed.
pub struct FileWriteTool;

#[derive(Debug, Deserialize)]
struct FileWriteInput {
    path: String,
    content: String,
}

#[async_trait]
impl Tool for FileWriteTool {
    #[inline]
    fn name(&self) -> &str {
        "FileWrite"
    }

    fn description(&self) -> &str {
        include_str!("prompts/file_write.txt")
    }

    fn input_schema(&self) -> ToolInputSchema {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Absolute or relative file path" },
                "content": { "type": "string", "description": "Full file content to write" }
            },
            "required": ["path", "content"]
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
        let params: FileWriteInput = serde_json::from_value(input)?;
        let path = fs::resolve_path(&ctx.cwd, &params.path);

        // Sensitive-file guard — use resolved absolute path to prevent relative-path bypass.
        let abs_path = if path.is_absolute() {
            path.clone()
        } else {
            ctx.cwd.join(&path)
        };
        if let Some(name) = abs_path.file_name().and_then(|n| n.to_str()) {
            for prefix in SENSITIVE_PREFIXES {
                if name.starts_with(prefix) {
                    return Ok(ToolResult::error(format!(
                        "Refusing to write to sensitive file: {name}"
                    )));
                }
            }
        }

        // Ensure parent directory exists.
        if let Some(parent) = path.parent() {
            fs::ensure_dir(parent)?;
        }

        let bytes = params.content.len();
        let line_count = params.content.lines().count();

        fs::atomic_write(&path, &params.content)?;

        Ok(ToolResult::text(format!(
            "Wrote {bytes} bytes ({line_count} lines) to {}",
            path.display()
        )))
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
    async fn write_new_file() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("out.txt");
        let tool = FileWriteTool;
        let result = tool
            .call(
                serde_json::json!({ "path": file.to_str().unwrap(), "content": "hello\nworld" }),
                &mut ctx(&dir),
            )
            .await
            .unwrap();
        assert!(!result.is_error);
        let written = std::fs::read_to_string(&file).unwrap();
        assert_eq!(written, "hello\nworld");
    }

    #[tokio::test]
    async fn auto_creates_directories() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("sub").join("dir").join("file.txt");
        let tool = FileWriteTool;
        let result = tool
            .call(
                serde_json::json!({ "path": file.to_str().unwrap(), "content": "ok" }),
                &mut ctx(&dir),
            )
            .await
            .unwrap();
        assert!(!result.is_error);
        assert!(file.exists());
    }

    #[tokio::test]
    async fn rejects_sensitive_env() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join(".env");
        let tool = FileWriteTool;
        let result = tool
            .call(
                serde_json::json!({ "path": file.to_str().unwrap(), "content": "SECRET=x" }),
                &mut ctx(&dir),
            )
            .await
            .unwrap();
        assert!(result.is_error);
        assert!(!file.exists());
    }

    #[tokio::test]
    async fn rejects_sensitive_ssh() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join(".ssh_config");
        let tool = FileWriteTool;
        let result = tool
            .call(
                serde_json::json!({ "path": file.to_str().unwrap(), "content": "key" }),
                &mut ctx(&dir),
            )
            .await
            .unwrap();
        assert!(result.is_error);
    }

    #[test]
    fn not_read_only() {
        let tool = FileWriteTool;
        assert!(!tool.is_read_only(&serde_json::json!({})));
    }

    #[test]
    fn needs_permission_ask() {
        let tool = FileWriteTool;
        assert_eq!(
            tool.needs_permission(&serde_json::json!({})),
            PermissionCheck::NeedsAsk
        );
    }

    #[tokio::test]
    async fn reports_byte_and_line_count() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("count.txt");
        let tool = FileWriteTool;
        let result = tool
            .call(
                serde_json::json!({ "path": file.to_str().unwrap(), "content": "a\nb\nc" }),
                &mut ctx(&dir),
            )
            .await
            .unwrap();
        let text = result.content[0].text.as_deref().unwrap();
        assert!(text.contains("5 bytes"));
        assert!(text.contains("3 lines"));
    }
}
