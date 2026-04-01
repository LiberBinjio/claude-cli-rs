//! `/model` — view or switch the active model.

use async_trait::async_trait;

use crate::command::{Command, CommandContext, CommandResult};

/// View or switch the active Claude model.
pub struct ModelCommand;

#[async_trait]
impl Command for ModelCommand {
    fn name(&self) -> &str {
        "model"
    }

    fn description(&self) -> &str {
        "View or change the current model"
    }

    fn usage(&self) -> &str {
        "/model [model_name]"
    }

    async fn execute(
        &self,
        args: &str,
        _ctx: &mut CommandContext,
    ) -> anyhow::Result<CommandResult> {
        let trimmed = args.trim();
        if trimmed.is_empty() {
            // CROSS-DEP: read from AppState.config when available
            Ok(CommandResult::Handled(Some(
                "Current model: claude-sonnet-4-20250514".to_string(),
            )))
        } else {
            // CROSS-DEP: update AppState.config when available
            Ok(CommandResult::Handled(Some(format!(
                "Model switched to: {trimmed}"
            ))))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_model_no_args() {
        let cmd = ModelCommand;
        let mut ctx = CommandContext {
            placeholder_state: (),
            event_tx: None,
        };
        let result = cmd.execute("", &mut ctx).await.unwrap();
        match result {
            CommandResult::Handled(Some(msg)) => assert!(msg.contains("Current model")),
            _ => panic!("expected model info"),
        }
    }

    #[tokio::test]
    async fn test_model_switch() {
        let cmd = ModelCommand;
        let mut ctx = CommandContext {
            placeholder_state: (),
            event_tx: None,
        };
        let result = cmd.execute("claude-opus-4-20250514", &mut ctx).await.unwrap();
        match result {
            CommandResult::Handled(Some(msg)) => {
                assert!(msg.contains("switched to"));
                assert!(msg.contains("claude-opus-4-20250514"));
            }
            _ => panic!("expected switch message"),
        }
    }
}
