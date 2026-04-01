//! `/init` — initialize project configuration.

use async_trait::async_trait;

use crate::command::{Command, CommandContext, CommandResult};

/// Initialize Claude Code for the current project.
pub struct InitCommand;

#[async_trait]
impl Command for InitCommand {
    fn name(&self) -> &str {
        "init"
    }

    fn description(&self) -> &str {
        "Initialize Claude Code in this project"
    }

    async fn execute(
        &self,
        _args: &str,
        _ctx: &mut CommandContext,
    ) -> anyhow::Result<CommandResult> {
        let cwd = std::env::current_dir().unwrap_or_default();
        let claude_md = cwd.join("CLAUDE.md");

        if claude_md.exists() {
            Ok(CommandResult::Handled(Some(
                "CLAUDE.md already exists in this directory.".to_string(),
            )))
        } else {
            Ok(CommandResult::SendToApi(
                "Create a CLAUDE.md file for this project. Analyze the codebase structure, \
                 identify the language, framework, build system, and testing approach. \
                 Write a concise CLAUDE.md with project conventions and key instructions."
                    .to_string(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_init_runs() {
        let cmd = InitCommand;
        let mut ctx = CommandContext {
            placeholder_state: (),
            event_tx: None,
        };
        let result = cmd.execute("", &mut ctx).await.unwrap();
        // Could be either Handled or SendToApi depending on CLAUDE.md existence
        match result {
            CommandResult::Handled(Some(_)) | CommandResult::SendToApi(_) => {}
            _ => panic!("unexpected result"),
        }
    }
}
