//! CLI argument definitions using clap derive.

use clap::{Parser, Subcommand};

/// Claude Code (Rust) — AI coding assistant in the terminal.
#[derive(Parser, Debug)]
#[command(
    name = "claude",
    version = "0.1.0",
    about = "Claude Code (Rust) — AI coding assistant"
)]
pub struct CliArgs {
    /// Initial prompt to send (non-interactive when combined with --print).
    pub prompt: Option<String>,

    /// Print the response and exit (non-interactive mode).
    #[arg(short, long)]
    pub print: bool,

    /// Model to use for conversations.
    #[arg(long, default_value = "claude-sonnet-4-20250514")]
    pub model: String,

    /// Working directory (defaults to current directory).
    #[arg(long)]
    pub cwd: Option<String>,

    /// Resume a previous session by ID.
    #[arg(long)]
    pub resume: Option<String>,

    /// Enable verbose/debug logging.
    #[arg(short, long)]
    pub verbose: bool,

    /// Use GitHub Copilot via Agent Maestro proxy (requires VS Code + Agent Maestro extension).
    #[arg(long)]
    pub copilot: bool,

    /// Subcommand to run.
    #[command(subcommand)]
    pub command: Option<CliCommand>,
}

/// Available subcommands.
#[derive(Subcommand, Debug)]
pub enum CliCommand {
    /// Run internal diagnostics.
    SelfTest,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_args() {
        let args = CliArgs::parse_from(["claude"]);
        assert!(args.prompt.is_none());
        assert!(!args.print);
        assert_eq!(args.model, "claude-sonnet-4-20250514");
        assert!(args.cwd.is_none());
        assert!(!args.verbose);
        assert!(args.command.is_none());
    }

    #[test]
    fn test_print_mode() {
        let args = CliArgs::parse_from(["claude", "-p", "hello"]);
        assert!(args.print);
        assert_eq!(args.prompt.as_deref(), Some("hello"));
    }

    #[test]
    fn test_model_override() {
        let args = CliArgs::parse_from(["claude", "--model", "claude-haiku-4-20250414"]);
        assert_eq!(args.model, "claude-haiku-4-20250414");
    }

    #[test]
    fn test_self_test_subcommand() {
        let args = CliArgs::parse_from(["claude", "self-test"]);
        assert!(matches!(args.command, Some(CliCommand::SelfTest)));
    }
}