//! Command trait and supporting types for the slash command system.

use async_trait::async_trait;
use tokio::sync::mpsc;

// CROSS-DEP: dev1 — when claude_core is ready, replace with:
//   use claude_core::state::AppState;
//   use std::sync::Arc;
//   use tokio::sync::RwLock;
//   pub state: Arc<RwLock<AppState>>,

/// Execution context passed to every command.
///
/// Placeholder fields will be replaced once `claude_core` types are available.
pub struct CommandContext {
    /// Placeholder: will become `Arc<RwLock<AppState>>` after dev1 finishes T06.
    pub placeholder_state: (),
    /// Optional channel for sending events back to the caller.
    pub event_tx: Option<mpsc::Sender<String>>,
}

/// The outcome of running a command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandResult {
    /// Command handled; optional message to display.
    Handled(Option<String>),
    /// Forward raw text to the API.
    SendToApi(String),
}

/// Trait that every slash command must implement.
#[async_trait]
pub trait Command: Send + Sync {
    /// Primary name used after the `/` prefix (e.g. `"help"`).
    fn name(&self) -> &str;

    /// Alternative names that also resolve to this command.
    fn aliases(&self) -> &[&str] {
        &[]
    }

    /// One-line description shown in `/help` output.
    fn description(&self) -> &str;

    /// Usage string, e.g. `/help [command_name]`.
    fn usage(&self) -> &str {
        ""
    }

    /// Hidden commands are omitted from the `/help` listing.
    fn is_hidden(&self) -> bool {
        false
    }

    /// Execute the command with the given arguments.
    async fn execute(
        &self,
        args: &str,
        ctx: &mut CommandContext,
    ) -> anyhow::Result<CommandResult>;
}
