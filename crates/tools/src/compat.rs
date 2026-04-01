//! CROSS-DEP: Placeholder types matching claude_core/claude_utils interfaces.
//! Remove this file and switch to real imports when dev1/dev2 restore their crates.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ---- claude_core::tool ----

/// Tool input schema (JSON Schema value).
pub type ToolInputSchema = serde_json::Value;

/// Result from a tool execution.
#[derive(Debug, Clone)]
#[must_use]
pub struct ToolResult {
    /// Content blocks.
    pub content: Vec<ToolResultContent>,
    /// Whether this result represents an error.
    pub is_error: bool,
}

impl ToolResult {
    /// Create a successful text result.
    #[inline]
    pub fn text(s: impl Into<String>) -> Self {
        Self {
            content: vec![ToolResultContent {
                content_type: "text".into(),
                text: Some(s.into()),
            }],
            is_error: false,
        }
    }

    /// Create an error text result.
    #[inline]
    pub fn error(s: impl Into<String>) -> Self {
        Self {
            content: vec![ToolResultContent {
                content_type: "text".into(),
                text: Some(s.into()),
            }],
            is_error: true,
        }
    }
}

/// Content within a tool result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultContent {
    /// Type of content (e.g. "text").
    #[serde(rename = "type")]
    pub content_type: String,
    /// Text content.
    pub text: Option<String>,
}

/// Context provided to a tool invocation.
#[derive(Debug, Clone)]
pub struct ToolUseContext {
    /// Current working directory.
    pub cwd: std::path::PathBuf,
    /// Active permission mode.
    pub permission_mode: PermissionMode,
    /// Unique ID of this tool use.
    pub tool_use_id: String,
    /// Session identifier.
    pub session_id: String,
}

/// Permission check outcome for a tool invocation.
#[derive(Debug, Clone, PartialEq)]
pub enum PermissionCheck {
    /// Tool may run without asking.
    Allowed,
    /// User must confirm.
    NeedsAsk,
    /// Denied with reason.
    Denied(String),
}

/// Permission mode governing tool execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PermissionMode {
    /// Default: read-only auto, write asks.
    Default,
    /// Plan mode: only read-only tools.
    Plan,
    /// Auto-edit: file tools auto-approved.
    AutoEdit,
    /// Full auto: everything auto-approved.
    FullAuto,
    /// Bypass all permission checks.
    BypassPermissions,
}

/// The core tool trait. Every tool in the system implements this.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Machine-readable tool name.
    fn name(&self) -> &str;
    /// Human-readable description.
    fn description(&self) -> &str;
    /// JSON Schema for the tool's input parameters.
    fn input_schema(&self) -> ToolInputSchema;
    /// Whether this tool is read-only for the given input.
    fn is_read_only(&self, input: &serde_json::Value) -> bool;
    /// Display name for UI.
    fn user_facing_name(&self) -> &str { self.name() }
    /// Permission check result for the given input.
    fn needs_permission(&self, _input: &serde_json::Value) -> PermissionCheck {
        PermissionCheck::NeedsAsk
    }
    /// Execute the tool with given input and context.
    async fn call(
        &self,
        input: serde_json::Value,
        ctx: &mut ToolUseContext,
    ) -> anyhow::Result<ToolResult>;
}

// ---- claude_utils::shell (minimal) ----

/// Result of a shell command execution.
#[derive(Debug, Clone)]
pub struct ShellResult {
    /// Combined stdout.
    pub stdout: String,
    /// Combined stderr.
    pub stderr: String,
    /// Exit code.
    pub exit_code: Option<i32>,
    /// Whether the command timed out.
    pub timed_out: bool,
}

/// Execute a shell command asynchronously with timeout.
pub async fn execute_shell(
    command: &str,
    cwd: &std::path::Path,
    timeout_secs: u64,
) -> anyhow::Result<ShellResult> {
    use tokio::process::Command;

    let (shell, flag) = if cfg!(windows) {
        ("cmd", "/C")
    } else {
        ("/bin/bash", "-c")
    };

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        Command::new(shell)
            .arg(flag)
            .arg(command)
            .current_dir(cwd)
            .output(),
    )
    .await;

    match result {
        Ok(Ok(output)) => Ok(ShellResult {
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            exit_code: output.status.code(),
            timed_out: false,
        }),
        Ok(Err(e)) => Err(anyhow::anyhow!("Failed to execute command: {e}")),
        Err(_) => Ok(ShellResult {
            stdout: String::new(),
            stderr: String::new(),
            exit_code: None,
            timed_out: true,
        }),
    }
}
