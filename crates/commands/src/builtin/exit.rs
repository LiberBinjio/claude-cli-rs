//! `/exit` — exit the application.

use async_trait::async_trait;

use crate::command::{Command, CommandContext, CommandResult};

/// Exit the application.
pub struct ExitCommand;

#[async_trait]
impl Command for ExitCommand {
    fn name(&self) -> &str {
        "exit"
    }

    fn aliases(&self) -> &[&str] {
        &["quit", "q"]
    }

    fn description(&self) -> &str {
        "Exit Claude Code"
    }

    async fn execute(
        &self,
        _args: &str,
        _ctx: &mut CommandContext,
    ) -> anyhow::Result<CommandResult> {
        // None signals exit to the caller
        Ok(CommandResult::Handled(None))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exit_metadata() {
        let cmd = ExitCommand;
        assert_eq!(cmd.name(), "exit");
        assert!(cmd.aliases().contains(&"quit"));
        assert!(cmd.aliases().contains(&"q"));
    }

    #[tokio::test]
    async fn test_exit_returns_none() {
        let cmd = ExitCommand;
        let mut ctx = CommandContext {
            placeholder_state: (),
            event_tx: None,
        };
        let result = cmd.execute("", &mut ctx).await.unwrap();
        assert_eq!(result, CommandResult::Handled(None));
    }
}
