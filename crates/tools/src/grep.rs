//! GrepTool: search files using ripgrep with grep/findstr fallback.

use async_trait::async_trait;
use claude_core::tool::{PermissionCheck, Tool, ToolInputSchema, ToolResult, ToolUseContext};
use claude_utils::shell::{execute_shell, ShellCommand};
use serde_json::Value;

const MAX_RESULT_LINES: usize = 200;

/// Tool for searching file contents by pattern.
pub struct GrepTool;

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &str { "Grep" }

    fn description(&self) -> &str {
        "Search files for a pattern using ripgrep (rg). Falls back to grep \
         or findstr if rg is not available."
    }

    fn input_schema(&self) -> ToolInputSchema {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Regex pattern to search for"
                },
                "path": {
                    "type": "string",
                    "description": "Directory or file to search in (default: current directory)"
                },
                "include": {
                    "type": "string",
                    "description": "File glob pattern to include (e.g. '*.rs')"
                }
            },
            "required": ["pattern"]
        })
    }

    fn is_read_only(&self, _input: &Value) -> bool { true }

    fn needs_permission(&self, _input: &Value) -> PermissionCheck {
        PermissionCheck::Allowed
    }

    async fn call(&self, input: Value, ctx: &mut ToolUseContext) -> anyhow::Result<ToolResult> {
        let pattern = input
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing 'pattern' parameter"))?;

        let path = input
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        let include = input.get("include").and_then(|v| v.as_str());

        // Try rg first, then platform-appropriate fallback
        let command = build_search_command(pattern, path, include);
        let shell_cmd = ShellCommand {
            command,
            cwd: Some(ctx.cwd.clone()),
            timeout: Some(std::time::Duration::from_secs(30)),
            env: std::collections::HashMap::new(),
        };
        let result = execute_shell(&shell_cmd).await?;

        if result.timed_out {
            return Ok(ToolResult::error("Search timed out after 30s"));
        }

        let output = if !result.stdout.is_empty() {
            &result.stdout
        } else if !result.stderr.is_empty() {
            return Ok(ToolResult::error(result.stderr));
        } else {
            return Ok(ToolResult::text("No matches found."));
        };

        // Truncate to MAX_RESULT_LINES
        let lines: Vec<&str> = output.lines().collect();
        let truncated = if lines.len() > MAX_RESULT_LINES {
            let joined: String = lines[..MAX_RESULT_LINES].join("\n");
            format!(
                "{joined}\n\n[Results truncated: showing {MAX_RESULT_LINES} of {} matches]",
                lines.len()
            )
        } else {
            output.to_string()
        };

        Ok(ToolResult::text(truncated))
    }
}

/// Build the search command string, preferring rg.
fn build_search_command(pattern: &str, path: &str, include: Option<&str>) -> String {
    // Escape pattern for shell safety
    let escaped = shell_escape(pattern);

    if cfg!(windows) {
        // Try rg first, fallback to findstr
        let mut cmd = format!("rg -n \"{escaped}\"");
        if let Some(glob) = include {
            cmd.push_str(&format!(" --glob \"{glob}\""));
        }
        cmd.push_str(&format!(" \"{path}\" 2>nul"));

        // Fallback: findstr /S /N searches recursively
        let findstr_glob = if let Some(glob) = include {
            format!("{path}\\{glob}")
        } else {
            format!("{path}\\*")
        };
        cmd.push_str(&format!(" || findstr /S /N \"{escaped}\" {findstr_glob}"));
        cmd
    } else {
        let mut cmd = format!("rg -n '{escaped}'");
        if let Some(glob) = include {
            cmd.push_str(&format!(" --glob '{glob}'"));
        }
        cmd.push_str(&format!(" '{path}' 2>/dev/null || grep -rn '{escaped}' '{path}'"));
        cmd
    }
}

/// Minimal shell escaping (remove dangerous chars).
fn shell_escape(s: &str) -> String {
    s.chars()
        .filter(|c| !matches!(c, '\'' | '"' | '`' | '$' | '!' | '\\'))
        .collect()
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
        let tool = GrepTool;
        let schema = tool.input_schema();
        assert_eq!(schema["required"][0], "pattern");
    }

    #[test]
    fn test_read_only() {
        let tool = GrepTool;
        assert!(tool.is_read_only(&serde_json::json!({})));
    }

    #[test]
    fn test_permission() {
        let tool = GrepTool;
        assert_eq!(
            tool.needs_permission(&serde_json::json!({})),
            PermissionCheck::Allowed
        );
    }

    #[test]
    fn test_build_command() {
        let cmd = build_search_command("TODO", "src", Some("*.rs"));
        assert!(cmd.contains("TODO"));
        assert!(cmd.contains("src"));
        assert!(cmd.contains("*.rs"));
    }

    #[test]
    fn test_shell_escape() {
        assert_eq!(shell_escape("hello"), "hello");
        assert_eq!(shell_escape("he'llo"), "hello");
        assert_eq!(shell_escape("$(rm -rf /)"), "(rm -rf /)");
    }

    #[tokio::test]
    async fn test_grep_search() {
        let tool = GrepTool;
        let mut ctx = test_ctx();
        ctx.cwd = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let input = serde_json::json!({
            "pattern": "GrepTool",
            "path": "src",
            "include": "*.rs"
        });
        let result = tool.call(input, &mut ctx).await.unwrap();
        assert!(!result.is_error);
        let text = result.content[0].text.as_deref().unwrap_or("");
        assert!(text.contains("GrepTool"));
    }

    #[tokio::test]
    async fn test_grep_missing_pattern() {
        let tool = GrepTool;
        let mut ctx = test_ctx();
        let input = serde_json::json!({});
        let result = tool.call(input, &mut ctx).await;
        assert!(result.is_err(), "missing pattern should return Err");
    }

    #[test]
    fn test_read_only_always() {
        let tool = GrepTool;
        // Grep is read-only regardless of input
        assert!(tool.is_read_only(&serde_json::json!({"pattern": "rm -rf /"})));
    }

    #[test]
    fn test_build_command_no_include() {
        let cmd = build_search_command("TODO", ".", None);
        assert!(cmd.contains("TODO"));
        assert!(!cmd.contains("--include"), "no include flag without pattern");
    }
}
