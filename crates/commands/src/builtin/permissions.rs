//! `/permissions` — display current permission mode and rules.

use async_trait::async_trait;

use crate::command::{Command, CommandContext, CommandResult};

/// Display current permission mode and rules.
pub struct PermissionsCommand;

#[async_trait]
impl Command for PermissionsCommand {
    fn name(&self) -> &str {
        "permissions"
    }

    fn aliases(&self) -> &[&str] {
        &["perms"]
    }

    fn description(&self) -> &str {
        "Show permission settings"
    }

    async fn execute(
        &self,
        _args: &str,
        _ctx: &mut CommandContext,
    ) -> anyhow::Result<CommandResult> {
        // CROSS-DEP: read permission_mode from AppState when available
        let info = "\
Permissions
  Mode: default
  Allowed tools: (none overridden)
  Denied tools: (none)";
        Ok(CommandResult::Handled(Some(info.to_string())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_permissions_output() {
        let cmd = PermissionsCommand;
        let mut ctx = CommandContext {
            placeholder_state: (),
            event_tx: None,
        };
        let result = cmd.execute("", &mut ctx).await.unwrap();
        match result {
            CommandResult::Handled(Some(msg)) => {
                assert!(msg.contains("Permissions"));
                assert!(msg.contains("Mode:"));
            }
            _ => panic!("expected permissions info"),
        }
    }
}
