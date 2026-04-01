//! `/status` — display current session status.

use async_trait::async_trait;

use crate::command::{Command, CommandContext, CommandResult};

/// Display current session status.
pub struct StatusCommand;

#[async_trait]
impl Command for StatusCommand {
    fn name(&self) -> &str {
        "status"
    }

    fn description(&self) -> &str {
        "Show current session status"
    }

    async fn execute(
        &self,
        _args: &str,
        _ctx: &mut CommandContext,
    ) -> anyhow::Result<CommandResult> {
        // CROSS-DEP: when AppState is available, read model/session_id/messages/cwd
        let status = "\
Model: claude-sonnet-4-20250514
Session: (pending AppState integration)
Messages: 0
Cwd: .";
        Ok(CommandResult::Handled(Some(status.to_string())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_status_output() {
        let cmd = StatusCommand;
        let mut ctx = CommandContext {
            placeholder_state: (),
            event_tx: None,
        };
        let result = cmd.execute("", &mut ctx).await.unwrap();
        match result {
            CommandResult::Handled(Some(msg)) => {
                assert!(msg.contains("Model:"));
                assert!(msg.contains("Session:"));
            }
            _ => panic!("expected status text"),
        }
    }
}
