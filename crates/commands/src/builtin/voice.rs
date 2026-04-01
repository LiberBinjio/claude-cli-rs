//! `/voice` — toggle voice input mode.

use async_trait::async_trait;

use crate::command::{Command, CommandContext, CommandResult};

/// Toggle voice input mode.
pub struct VoiceCommand;

#[async_trait]
impl Command for VoiceCommand {
    fn name(&self) -> &str {
        "voice"
    }

    fn description(&self) -> &str {
        "Toggle voice mode"
    }

    fn is_hidden(&self) -> bool {
        true
    }

    async fn execute(
        &self,
        _args: &str,
        _ctx: &mut CommandContext,
    ) -> anyhow::Result<CommandResult> {
        // CROSS-DEP: toggle voice_mode flag in AppState when available
        Ok(CommandResult::Handled(Some(
            "Voice mode toggled. (pending AppState integration)".to_string(),
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_voice_toggle() {
        let cmd = VoiceCommand;
        let mut ctx = CommandContext {
            placeholder_state: (),
            event_tx: None,
        };
        let result = cmd.execute("", &mut ctx).await.unwrap();
        assert!(matches!(result, CommandResult::Handled(Some(_))));
    }
}
