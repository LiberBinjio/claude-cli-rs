//! `/session` — display session information.

use async_trait::async_trait;

use crate::command::{Command, CommandContext, CommandResult};

/// Display current session information.
pub struct SessionCommand;

#[async_trait]
impl Command for SessionCommand {
    fn name(&self) -> &str {
        "session"
    }

    fn description(&self) -> &str {
        "Show session information"
    }

    async fn execute(
        &self,
        _args: &str,
        _ctx: &mut CommandContext,
    ) -> anyhow::Result<CommandResult> {
        // CROSS-DEP: read session_id/timestamps from AppState when available
        let info = "Session Information\n  ID: (pending AppState)\n  Messages: 0";
        Ok(CommandResult::Handled(Some(info.to_string())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_output() {
        let cmd = SessionCommand;
        let mut ctx = CommandContext {
            placeholder_state: (),
            event_tx: None,
        };
        let result = cmd.execute("", &mut ctx).await.unwrap();
        match result {
            CommandResult::Handled(Some(msg)) => assert!(msg.contains("Session")),
            _ => panic!("expected session info"),
        }
    }
}
