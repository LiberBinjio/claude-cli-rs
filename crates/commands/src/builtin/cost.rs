//! `/cost` — display token usage and cost statistics.

use async_trait::async_trait;

use crate::command::{Command, CommandContext, CommandResult};

/// Display token usage and estimated cost.
pub struct CostCommand;

#[async_trait]
impl Command for CostCommand {
    fn name(&self) -> &str {
        "cost"
    }

    fn description(&self) -> &str {
        "Show token usage and cost"
    }

    async fn execute(
        &self,
        _args: &str,
        _ctx: &mut CommandContext,
    ) -> anyhow::Result<CommandResult> {
        // CROSS-DEP: when AppState is available, read actual token/cost counters
        let report = "\
Token Usage
  Input tokens:  0
  Output tokens: 0
  Total tokens:  0

Estimated Cost: $0.0000";
        Ok(CommandResult::Handled(Some(report.to_string())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cost_output() {
        let cmd = CostCommand;
        let mut ctx = CommandContext {
            placeholder_state: (),
            event_tx: None,
        };
        let result = cmd.execute("", &mut ctx).await.unwrap();
        match result {
            CommandResult::Handled(Some(msg)) => {
                assert!(msg.contains("Token Usage"));
                assert!(msg.contains("Estimated Cost"));
            }
            _ => panic!("expected cost report"),
        }
    }
}
