//! Tool trait and related types for tool execution.

use crate::message::ToolResultContent;
use crate::permission::PermissionMode;
use async_trait::async_trait;
use serde_json::Value;

/// Schema describing a tool's expected input (JSON Schema).
pub type ToolInputSchema = Value;

/// The result of executing a tool.
#[derive(Debug, Clone)]
pub struct ToolResult {
    /// The result content blocks.
    pub content: Vec<ToolResultContent>,
    /// Whether this result represents an error.
    pub is_error: bool,
}

impl ToolResult {
    /// Create a successful text result.
    #[must_use]
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
    #[must_use]
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

/// Context available to a tool during execution.
#[derive(Debug, Clone)]
pub struct ToolUseContext {
    /// Current working directory.
    pub cwd: std::path::PathBuf,
    /// Active permission mode.
    pub permission_mode: PermissionMode,
    /// Unique ID for this tool invocation.
    pub tool_use_id: String,
    /// Current session ID.
    pub session_id: String,
}

/// Result of a permission check for a tool.
#[derive(Debug, Clone, PartialEq)]
pub enum PermissionCheck {
    /// Tool is allowed to proceed.
    Allowed,
    /// Tool needs user confirmation.
    NeedsAsk,
    /// Tool is denied with a reason.
    Denied(String),
}

/// Trait for implementing a tool callable by the AI assistant.
#[async_trait]
pub trait Tool: Send + Sync {
    /// The canonical name of this tool.
    fn name(&self) -> &str;

    /// A human-readable description of what this tool does.
    fn description(&self) -> &str;

    /// JSON Schema describing the expected input.
    fn input_schema(&self) -> ToolInputSchema;

    /// Whether this tool is read-only for the given input.
    fn is_read_only(&self, input: &Value) -> bool;

    /// The user-facing display name (defaults to `name()`).
    fn user_facing_name(&self) -> &str {
        self.name()
    }

    /// Check if this tool needs permission for the given input.
    fn needs_permission(&self, _input: &Value) -> PermissionCheck {
        PermissionCheck::NeedsAsk
    }

    /// Execute the tool with the given input and context.
    async fn call(&self, input: Value, ctx: &mut ToolUseContext) -> anyhow::Result<ToolResult>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_result_text() {
        let r = ToolResult::text("hello");
        assert!(!r.is_error);
        assert_eq!(r.content.len(), 1);
        assert_eq!(r.content[0].text.as_deref(), Some("hello"));
        assert_eq!(r.content[0].content_type, "text");
    }

    #[test]
    fn test_tool_result_error() {
        let r = ToolResult::error("failed");
        assert!(r.is_error);
        assert_eq!(r.content[0].text.as_deref(), Some("failed"));
    }

    #[test]
    fn test_permission_check_equality() {
        assert_eq!(PermissionCheck::Allowed, PermissionCheck::Allowed);
        assert_eq!(PermissionCheck::NeedsAsk, PermissionCheck::NeedsAsk);
        assert_ne!(PermissionCheck::Allowed, PermissionCheck::NeedsAsk);
        assert_eq!(
            PermissionCheck::Denied("no".into()),
            PermissionCheck::Denied("no".into())
        );
    }
}
