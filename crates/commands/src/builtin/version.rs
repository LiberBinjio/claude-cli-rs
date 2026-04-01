//! `/version` — display the current version.

use async_trait::async_trait;

use crate::command::{Command, CommandContext, CommandResult};

/// Display the application version.
pub struct VersionCommand;

#[async_trait]
impl Command for VersionCommand {
    fn name(&self) -> &str {
        "version"
    }

    fn aliases(&self) -> &[&str] {
        &["v"]
    }

    fn description(&self) -> &str {
        "Show version information"
    }

    async fn execute(
        &self,
        _args: &str,
        _ctx: &mut CommandContext,
    ) -> anyhow::Result<CommandResult> {
        let version = env!("CARGO_PKG_VERSION");
        Ok(CommandResult::Handled(Some(format!(
            "Claude Code v{version} (Rust)"
        ))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_version_output() {
        let cmd = VersionCommand;
        let mut ctx = CommandContext {
            placeholder_state: (),
            event_tx: None,
        };
        let result = cmd.execute("", &mut ctx).await.unwrap();
        match result {
            CommandResult::Handled(Some(msg)) => {
                assert!(msg.contains("Claude Code v"));
                assert!(msg.contains("Rust"));
            }
            _ => panic!("expected version string"),
        }
    }
}
