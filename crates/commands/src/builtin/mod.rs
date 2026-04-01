//! Built-in slash commands.

pub mod help;
pub mod exit;
pub mod clear;
pub mod version;
pub mod status;
pub mod cost;
pub mod model;
pub mod compact;
pub mod config;
pub mod memory;
pub mod theme;
pub mod diff;
pub mod commit;
pub mod session;
pub mod resume;
pub mod permissions;
pub mod init;
pub mod mcp;
pub mod vim;
pub mod voice;

use std::sync::Arc;

use crate::registry::CommandRegistry;

/// Register all built-in commands into the given registry.
pub fn register_builtins(registry: &mut CommandRegistry) {
    registry.register(Arc::new(help::HelpCommand));
    registry.register(Arc::new(exit::ExitCommand));
    registry.register(Arc::new(clear::ClearCommand));
    registry.register(Arc::new(version::VersionCommand));
    registry.register(Arc::new(status::StatusCommand));
    registry.register(Arc::new(cost::CostCommand));
    registry.register(Arc::new(model::ModelCommand));
    registry.register(Arc::new(compact::CompactCommand));
    registry.register(Arc::new(config::ConfigCommand));
    registry.register(Arc::new(memory::MemoryCommand));
    registry.register(Arc::new(theme::ThemeCommand));
    registry.register(Arc::new(diff::DiffCommand));
    registry.register(Arc::new(commit::CommitCommand));
    registry.register(Arc::new(session::SessionCommand));
    registry.register(Arc::new(resume::ResumeCommand));
    registry.register(Arc::new(permissions::PermissionsCommand));
    registry.register(Arc::new(init::InitCommand));
    registry.register(Arc::new(mcp::McpCommand));
    registry.register(Arc::new(vim::VimCommand));
    registry.register(Arc::new(voice::VoiceCommand));
}
