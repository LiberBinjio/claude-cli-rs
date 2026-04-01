//! `/commit` — generate a commit message via the API.

use async_trait::async_trait;

use crate::command::{Command, CommandContext, CommandResult};

/// Ask Claude to generate a commit message from the staged diff.
pub struct CommitCommand;

#[async_trait]
impl Command for CommitCommand {
    fn name(&self) -> &str {
        "commit"
    }

    fn description(&self) -> &str {
        "Generate a commit message for staged changes"
    }

    async fn execute(
        &self,
        _args: &str,
        _ctx: &mut CommandContext,
    ) -> anyhow::Result<CommandResult> {
        let cwd = std::env::current_dir().unwrap_or_default();
        let diff = std::process::Command::new("git")
            .current_dir(&cwd)
            .args(["diff", "--staged"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default();

        if diff.is_empty() {
            return Ok(CommandResult::Handled(Some(
                "No staged changes found. Use `git add` first.".to_string(),
            )));
        }

        let prompt = format!(
            "Generate a concise, conventional commit message for the following staged diff. \
             Return ONLY the commit message, no explanation.\n\n```diff\n{diff}\n```"
        );
        Ok(CommandResult::SendToApi(prompt))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_commit_no_staged() {
        let cmd = CommitCommand;
        let mut ctx = CommandContext {
            placeholder_state: (),
            event_tx: None,
        };
        let result = cmd.execute("", &mut ctx).await.unwrap();
        // Either returns error or "no staged changes"
        assert!(matches!(result, CommandResult::Handled(Some(_))));
    }
}
