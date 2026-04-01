//! `/help` — display available commands and their descriptions.

use async_trait::async_trait;

use crate::command::{Command, CommandContext, CommandResult};

/// Show available commands or detailed help for a specific command.
pub struct HelpCommand;

#[async_trait]
impl Command for HelpCommand {
    fn name(&self) -> &str {
        "help"
    }

    fn aliases(&self) -> &[&str] {
        &["h", "?"]
    }

    fn description(&self) -> &str {
        "Show available commands"
    }

    fn usage(&self) -> &str {
        "/help [command_name]"
    }

    async fn execute(
        &self,
        args: &str,
        _ctx: &mut CommandContext,
    ) -> anyhow::Result<CommandResult> {
        let args = args.trim();

        if args.is_empty() {
            // General overview — list all visible commands.
            let text = generate_overview();
            Ok(CommandResult::Handled(Some(text)))
        } else {
            // Detailed help for a single command.
            let text = format!("Help for '{}': use /help to see all commands.", args);
            Ok(CommandResult::Handled(Some(text)))
        }
    }
}

/// Generate a quick overview of *all* commands registered with a temporary
/// registry. Because `HelpCommand::execute` does not receive the registry,
/// we build an overview from the hardcoded list.
fn generate_overview() -> String {
    let mut lines = vec!["Available commands:".to_owned()];
    // List known builtins statically — the real implementation will iterate
    // the registry once CommandContext carries a reference to it.
    let builtins: &[(&str, &str)] = &[
        ("help", "Show available commands"),
        ("exit", "Exit the application"),
        ("clear", "Clear conversation history"),
        ("version", "Show version information"),
        ("status", "Show session status"),
        ("cost", "Show token usage and cost"),
        ("model", "View or change the current model"),
        ("compact", "Compact conversation context"),
        ("config", "View configuration"),
        ("memory", "Show CLAUDE.md memory status"),
        ("theme", "Change color theme"),
        ("diff", "Show git diff"),
        ("commit", "Generate commit message"),
        ("session", "Session information"),
        ("resume", "Resume a previous session"),
        ("permissions", "View/change permission mode"),
        ("init", "Initialize project settings"),
        ("mcp", "MCP server status"),
        ("vim", "Toggle vim mode"),
        ("voice", "Toggle voice mode"),
    ];
    for (name, desc) in builtins {
        lines.push(format!("  /{name:<14} {desc}"));
    }
    lines.push(String::new());
    lines.push("Type /help <command> for more details.".to_owned());
    lines.join("\n")
}
