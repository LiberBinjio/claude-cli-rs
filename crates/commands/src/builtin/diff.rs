//! `/diff` — show current git diff.

use async_trait::async_trait;

use crate::command::{Command, CommandContext, CommandResult};

/// Display current git diff.
pub struct DiffCommand;

#[async_trait]
impl Command for DiffCommand {
    fn name(&self) -> &str {
        "diff"
    }

    fn description(&self) -> &str {
        "Show current git diff"
    }

    async fn execute(
        &self,
        args: &str,
        _ctx: &mut CommandContext,
    ) -> anyhow::Result<CommandResult> {
        let cwd = std::env::current_dir().unwrap_or_default();
        let staged = args.trim() == "--staged";

        let mut cmd = std::process::Command::new("git");
        cmd.current_dir(&cwd).arg("diff");
        if staged {
            cmd.arg("--staged");
        }
        match cmd.output() {
            Ok(output) if output.status.success() => {
                let diff = String::from_utf8_lossy(&output.stdout).to_string();
                if diff.is_empty() {
                    Ok(CommandResult::Handled(Some(
                        "No changes detected.".to_string(),
                    )))
                } else {
                    Ok(CommandResult::Handled(Some(diff)))
                }
            }
            Ok(output) => Ok(CommandResult::Handled(Some(format!(
                "git diff error: {}",
                String::from_utf8_lossy(&output.stderr)
            )))),
            Err(e) => Ok(CommandResult::Handled(Some(format!(
                "Failed to run git: {e}"
            )))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_diff_runs() {
        let cmd = DiffCommand;
        let mut ctx = CommandContext {
            placeholder_state: (),
            event_tx: None,
        };
        // In a non-git dir this returns an error message, which is fine
        let result = cmd.execute("", &mut ctx).await.unwrap();
        assert!(matches!(result, CommandResult::Handled(Some(_))));
    }
}
