//! `/config` — display current configuration.

use async_trait::async_trait;

use crate::command::{Command, CommandContext, CommandResult};

/// Display current configuration settings.
pub struct ConfigCommand;

#[async_trait]
impl Command for ConfigCommand {
    fn name(&self) -> &str {
        "config"
    }

    fn description(&self) -> &str {
        "Show current configuration"
    }

    async fn execute(
        &self,
        _args: &str,
        _ctx: &mut CommandContext,
    ) -> anyhow::Result<CommandResult> {
        // CROSS-DEP: read from AppState.config when available
        let info = "\
Configuration
  Model: claude-sonnet-4-20250514
  Permission mode: default
  Verbose: false";
        Ok(CommandResult::Handled(Some(info.to_string())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_config_output() {
        let cmd = ConfigCommand;
        let mut ctx = CommandContext {
            placeholder_state: (),
            event_tx: None,
        };
        let result = cmd.execute("", &mut ctx).await.unwrap();
        match result {
            CommandResult::Handled(Some(msg)) => {
                assert!(msg.contains("Configuration"));
                assert!(msg.contains("Model:"));
            }
            _ => panic!("expected config output"),
        }
    }
}
