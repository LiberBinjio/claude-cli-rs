//! Claude Code — core types (message, tool, permission, config, state, task)

pub mod config;
pub mod message;
pub mod permission;
pub mod state;
pub mod task;
pub mod tool;

// Re-export primary types for convenience.
pub use config::{AppConfig, McpServerConfig};
pub use message::{CacheControl, ContentBlock, ImageSource, Message, Role, ToolResultContent};
pub use permission::{check_permission, PermissionDecision, PermissionMode, PermissionRule};
pub use state::AppState;
pub use task::{Task, TaskStatus};
pub use tool::{PermissionCheck, Tool, ToolInputSchema, ToolResult, ToolUseContext};
