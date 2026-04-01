//! `/vim` — toggle vim keybinding mode.

use async_trait::async_trait;

use crate::command::{Command, CommandContext, CommandResult};

/// Toggle vim keybinding mode.
pub struct VimCommand;

#[async_trait]
impl Command for VimCommand {
    fn name(&self) -> &str {
        "vim"
    }

    fn description(&self) -> &str {
        "Toggle vim mode"
    }

    fn is_hidden(&self) -> bool {
        true
    }

    async fn execute(
        &self,
        _args: &str,
        _ctx: &mut CommandContext,
    ) -> anyhow::Result<CommandResult> {
        // CROSS-DEP: toggle vim_mode flag in AppState when available
        Ok(CommandResult::Handled(Some(
            "Vim mode toggled. (pending AppState integration)".to_string(),
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_vim_toggle() {
        let cmd = VimCommand;
        let mut ctx = CommandContext {
            placeholder_state: (),
            event_tx: None,
        };
        let result = cmd.execute("", &mut ctx).await.unwrap();
        assert!(matches!(result, CommandResult::Handled(Some(_))));
    }
}
