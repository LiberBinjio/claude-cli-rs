//! BashTool: execute shell commands with read-only detection and timeout.

use async_trait::async_trait;
use claude_core::tool::{PermissionCheck, Tool, ToolInputSchema, ToolResult, ToolUseContext};
use claude_utils::shell::{execute_shell, ShellCommand};
use serde_json::Value;
use std::collections::HashSet;
use std::sync::OnceLock;

const DEFAULT_TIMEOUT: u64 = 120;
const MAX_OUTPUT_CHARS: usize = 500_000;

/// Tool for executing shell commands.
pub struct BashTool;

/// Set of command prefixes considered read-only.
fn read_only_commands() -> &'static HashSet<&'static str> {
    static CMDS: OnceLock<HashSet<&str>> = OnceLock::new();
    CMDS.get_or_init(|| {
        [
            "cat", "ls", "echo", "pwd", "which", "where", "head", "tail",
            "wc", "find", "grep", "rg", "ag", "fd", "file", "stat", "du",
            "df", "date", "whoami", "hostname", "printenv", "env", "type",
            "less", "more", "tree", "bat", "exa", "eza", "diff", "sort",
            "uniq", "cut", "tr", "awk", "sed", "test", "dir",
        ]
        .into_iter()
        .collect()
    })
}

/// Operators/commands that indicate a write operation.
const WRITE_INDICATORS: &[&str] = &[
    ">", ">>", "rm ", "rmdir ", "mv ", "cp ", "mkdir ", "chmod ",
    "chown ", "ln ", "touch ", "tee ", "dd ", "mkfs", "fdisk",
    "apt ", "yum ", "pip ", "npm ", "cargo install", "sudo ",
];

/// Check if a bash command is read-only.
#[must_use]
pub fn is_read_only_bash_command(command: &str) -> bool {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return true;
    }

    // Check for write indicators
    for indicator in WRITE_INDICATORS {
        if trimmed.contains(indicator) {
            return false;
        }
    }

    // Handle pipes: all segments must start with read-only commands
    if trimmed.contains('|') {
        return trimmed.split('|').all(|segment| {
            let seg = segment.trim();
            let first_word = seg.split_whitespace().next().unwrap_or("");
            // Strip path prefix (e.g., /usr/bin/cat -> cat)
            let cmd_name = first_word.rsplit('/').next().unwrap_or(first_word);
            read_only_commands().contains(cmd_name)
        });
    }

    // Single command: check first word
    let first_word = trimmed.split_whitespace().next().unwrap_or("");
    let cmd_name = first_word.rsplit('/').next().unwrap_or(first_word);
    read_only_commands().contains(cmd_name)
}

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str { "Bash" }

    fn description(&self) -> &str {
        "Execute a shell command. Use for running scripts, installing packages, \
         compiling code, or any system command."
    }

    fn input_schema(&self) -> ToolInputSchema {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute"
                },
                "timeout": {
                    "type": "integer",
                    "description": "Timeout in seconds (default: 120)"
                }
            },
            "required": ["command"]
        })
    }

    fn is_read_only(&self, input: &Value) -> bool {
        input
            .get("command")
            .and_then(|v| v.as_str())
            .map(is_read_only_bash_command)
            .unwrap_or(false)
    }

    fn needs_permission(&self, input: &Value) -> PermissionCheck {
        if self.is_read_only(input) {
            PermissionCheck::Allowed
        } else {
            PermissionCheck::NeedsAsk
        }
    }

    async fn call(&self, input: Value, ctx: &mut ToolUseContext) -> anyhow::Result<ToolResult> {
        let command = input
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing 'command' parameter"))?;

        let timeout = input
            .get("timeout")
            .and_then(|v| v.as_u64())
            .unwrap_or(DEFAULT_TIMEOUT);

        let shell_cmd = ShellCommand {
            command: command.to_string(),
            cwd: Some(ctx.cwd.clone()),
            timeout: Some(std::time::Duration::from_secs(timeout)),
            env: std::collections::HashMap::new(),
        };
        let result = execute_shell(&shell_cmd).await?;

        if result.timed_out {
            return Ok(ToolResult::error(format!(
                "Command timed out after {timeout}s"
            )));
        }

        let mut output = String::new();
        if !result.stdout.is_empty() {
            output.push_str(&result.stdout);
        }
        if !result.stderr.is_empty() {
            if !output.is_empty() {
                output.push('\n');
            }
            output.push_str(&result.stderr);
        }
        if output.is_empty() {
            output.push_str("(no output)");
        }

        // Truncate overly long output
        if output.len() > MAX_OUTPUT_CHARS {
            output.truncate(MAX_OUTPUT_CHARS);
            output.push_str("\n...[output truncated]");
        }

        if result.exit_code.is_some_and(|c| c != 0) {
            Ok(ToolResult::error(output))
        } else {
            Ok(ToolResult::text(output))
        }
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
    fn test_read_only_simple() {
        assert!(is_read_only_bash_command("cat foo.txt"));
        assert!(is_read_only_bash_command("ls -la"));
        assert!(is_read_only_bash_command("echo hello"));
        assert!(is_read_only_bash_command("pwd"));
        assert!(is_read_only_bash_command("grep foo bar.txt"));
    }

    #[test]
    fn test_not_read_only() {
        assert!(!is_read_only_bash_command("rm -rf /"));
        assert!(!is_read_only_bash_command("echo x > file.txt"));
        assert!(!is_read_only_bash_command("mkdir newdir"));
        assert!(!is_read_only_bash_command("cp a b"));
    }

    #[test]
    fn test_pipe_read_only() {
        assert!(is_read_only_bash_command("cat file | wc -l"));
        assert!(is_read_only_bash_command("cat file | grep foo | sort"));
        assert!(!is_read_only_bash_command("cat file | tee output.txt"));
    }

    #[test]
    fn test_empty_command() {
        assert!(is_read_only_bash_command(""));
        assert!(is_read_only_bash_command("  "));
    }

    #[test]
    fn test_schema() {
        let tool = BashTool;
        let schema = tool.input_schema();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["command"].is_object());
        assert_eq!(schema["required"][0], "command");
    }

    #[test]
    fn test_permission_check() {
        let tool = BashTool;
        let ro_input = serde_json::json!({"command": "ls -la"});
        assert_eq!(tool.needs_permission(&ro_input), PermissionCheck::Allowed);

        let rw_input = serde_json::json!({"command": "rm file"});
        assert_eq!(tool.needs_permission(&rw_input), PermissionCheck::NeedsAsk);
    }

    #[tokio::test]
    async fn test_echo_command() {
        let tool = BashTool;
        let mut ctx = test_ctx();
        let input = serde_json::json!({"command": "echo hello_world"});
        let result = tool.call(input, &mut ctx).await.unwrap();
        assert!(!result.is_error);
        let text = result.content[0].text.as_deref().unwrap_or("");
        assert!(text.contains("hello_world"));
    }

    #[tokio::test]
    async fn test_timeout() {
        let tool = BashTool;
        let mut ctx = test_ctx();
        // Command that should time out in 1 second
        let cmd = if cfg!(windows) {
            "ping -n 10 127.0.0.1"
        } else {
            "sleep 10"
        };
        let input = serde_json::json!({"command": cmd, "timeout": 1});
        let result = tool.call(input, &mut ctx).await.unwrap();
        assert!(result.is_error);
        let text = result.content[0].text.as_deref().unwrap_or("");
        assert!(text.contains("timed out"));
    }
}
