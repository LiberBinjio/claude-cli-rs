//! `/mcp` — show MCP server status.

use async_trait::async_trait;

use crate::command::{Command, CommandContext, CommandResult};

/// Display MCP server connection status.
pub struct McpCommand;

#[async_trait]
impl Command for McpCommand {
    fn name(&self) -> &str {
        "mcp"
    }

    fn description(&self) -> &str {
        "Show MCP server status"
    }

    async fn execute(
        &self,
        _args: &str,
        _ctx: &mut CommandContext,
    ) -> anyhow::Result<CommandResult> {
        // CROSS-DEP: read from McpConnectionManager when available
        Ok(CommandResult::Handled(Some(
            "MCP Servers\n  No servers connected.".to_string(),
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mcp_output() {
        let cmd = McpCommand;
        let mut ctx = CommandContext {
            placeholder_state: (),
            event_tx: None,
        };
        let result = cmd.execute("", &mut ctx).await.unwrap();
        match result {
            CommandResult::Handled(Some(msg)) => assert!(msg.contains("MCP")),
            _ => panic!("expected mcp status"),
        }
    }
}
