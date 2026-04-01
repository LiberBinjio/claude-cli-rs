//! `/theme` — switch color theme.

use async_trait::async_trait;

use crate::command::{Command, CommandContext, CommandResult};

/// Switch the terminal color theme.
pub struct ThemeCommand;

#[async_trait]
impl Command for ThemeCommand {
    fn name(&self) -> &str {
        "theme"
    }

    fn description(&self) -> &str {
        "Switch color theme"
    }

    fn usage(&self) -> &str {
        "/theme [dark|light|auto]"
    }

    async fn execute(
        &self,
        args: &str,
        _ctx: &mut CommandContext,
    ) -> anyhow::Result<CommandResult> {
        let trimmed = args.trim();
        if trimmed.is_empty() {
            Ok(CommandResult::Handled(Some(
                "Available themes: dark, light, auto\nCurrent: dark".to_string(),
            )))
        } else {
            match trimmed {
                "dark" | "light" | "auto" => Ok(CommandResult::Handled(Some(format!(
                    "Theme switched to: {trimmed}"
                )))),
                _ => Ok(CommandResult::Handled(Some(format!(
                    "Unknown theme '{trimmed}'. Available: dark, light, auto"
                )))),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_theme_list() {
        let cmd = ThemeCommand;
        let mut ctx = CommandContext {
            placeholder_state: (),
            event_tx: None,
        };
        let result = cmd.execute("", &mut ctx).await.unwrap();
        match result {
            CommandResult::Handled(Some(msg)) => assert!(msg.contains("Available themes")),
            _ => panic!("expected theme list"),
        }
    }

    #[tokio::test]
    async fn test_theme_switch() {
        let cmd = ThemeCommand;
        let mut ctx = CommandContext {
            placeholder_state: (),
            event_tx: None,
        };
        let result = cmd.execute("light", &mut ctx).await.unwrap();
        match result {
            CommandResult::Handled(Some(msg)) => assert!(msg.contains("switched to: light")),
            _ => panic!("expected switch"),
        }
    }
}
