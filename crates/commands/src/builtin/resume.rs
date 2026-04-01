//! `/resume` — resume a previous session.

use async_trait::async_trait;

use crate::command::{Command, CommandContext, CommandResult};

/// Resume a past conversation session.
pub struct ResumeCommand;

#[async_trait]
impl Command for ResumeCommand {
    fn name(&self) -> &str {
        "resume"
    }

    fn description(&self) -> &str {
        "Resume a previous session"
    }

    fn usage(&self) -> &str {
        "/resume [session_id]"
    }

    async fn execute(
        &self,
        args: &str,
        _ctx: &mut CommandContext,
    ) -> anyhow::Result<CommandResult> {
        let trimmed = args.trim();
        if trimmed.is_empty() {
            // CROSS-DEP: list sessions from claude_services::session when deps available
            Ok(CommandResult::Handled(Some(
                "Usage: /resume <session_id>\n\
                 Session listing requires claude_services integration."
                    .to_string(),
            )))
        } else {
            // CROSS-DEP: actually load session into AppState when available
            Ok(CommandResult::Handled(Some(format!(
                "Resuming session: {trimmed} (pending AppState integration)"
            ))))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_resume_no_args() {
        let cmd = ResumeCommand;
        let mut ctx = CommandContext {
            placeholder_state: (),
            event_tx: None,
        };
        let result = cmd.execute("", &mut ctx).await.unwrap();
        assert!(matches!(result, CommandResult::Handled(Some(_))));
    }

    #[tokio::test]
    async fn test_resume_with_id() {
        let cmd = ResumeCommand;
        let mut ctx = CommandContext {
            placeholder_state: (),
            event_tx: None,
        };
        let result = cmd.execute("abc123", &mut ctx).await.unwrap();
        match result {
            CommandResult::Handled(Some(msg)) => assert!(msg.contains("abc123")),
            _ => panic!("expected resume message"),
        }
    }
}
