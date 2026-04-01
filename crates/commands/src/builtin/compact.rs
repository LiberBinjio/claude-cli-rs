//! `/compact` — trigger context compaction.

use async_trait::async_trait;

use crate::command::{Command, CommandContext, CommandResult};

/// Compress conversation history to save tokens.
pub struct CompactCommand;

#[async_trait]
impl Command for CompactCommand {
    fn name(&self) -> &str {
        "compact"
    }

    fn description(&self) -> &str {
        "Compact conversation to reduce token usage"
    }

    async fn execute(
        &self,
        _args: &str,
        _ctx: &mut CommandContext,
    ) -> anyhow::Result<CommandResult> {
        Ok(CommandResult::SendToApi(
            "Please provide a concise summary of our conversation so far, \
             focusing on key decisions and current task state."
                .to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_compact_sends_to_api() {
        let cmd = CompactCommand;
        let mut ctx = CommandContext {
            placeholder_state: (),
            event_tx: None,
        };
        let result = cmd.execute("", &mut ctx).await.unwrap();
        match result {
            CommandResult::SendToApi(prompt) => assert!(prompt.contains("summary")),
            _ => panic!("expected SendToApi"),
        }
    }
}
