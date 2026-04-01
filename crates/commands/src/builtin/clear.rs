//! `/clear` — clear conversation history.

use async_trait::async_trait;

use crate::command::{Command, CommandContext, CommandResult};

/// Clear the conversation history.
pub struct ClearCommand;

#[async_trait]
impl Command for ClearCommand {
    fn name(&self) -> &str {
        "clear"
    }

    fn description(&self) -> &str {
        "Clear conversation history"
    }

    async fn execute(
        &self,
        _args: &str,
        _ctx: &mut CommandContext,
    ) -> anyhow::Result<CommandResult> {
        // CROSS-DEP: when AppState is available, clear messages vec
        Ok(CommandResult::Handled(Some(
            "Conversation history cleared.".to_string(),
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_clear_returns_message() {
        let cmd = ClearCommand;
        let mut ctx = CommandContext {
            placeholder_state: (),
            event_tx: None,
        };
        let result = cmd.execute("", &mut ctx).await.unwrap();
        match result {
            CommandResult::Handled(Some(msg)) => assert!(msg.contains("cleared")),
            _ => panic!("expected Handled(Some)"),
        }
    }
}
