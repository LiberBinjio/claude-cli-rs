//! Claude Code — slash command system.

pub mod builtin;
pub mod command;
pub mod registry;

pub use command::{Command, CommandContext, CommandResult};
pub use registry::CommandRegistry;
