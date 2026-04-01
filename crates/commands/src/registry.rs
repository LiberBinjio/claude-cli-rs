//! Command registry — stores and resolves slash commands.

use std::sync::Arc;

use crate::command::Command;

/// Central registry for all registered slash commands.
pub struct CommandRegistry {
    commands: Vec<Arc<dyn Command>>,
}

impl CommandRegistry {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    /// Register a new command.
    pub fn register(&mut self, cmd: Arc<dyn Command>) {
        self.commands.push(cmd);
    }

    /// Parse `"/command args"` and return the matching command plus the remaining args.
    ///
    /// Returns `None` when the input does not start with `/` or no command matches.
    /// Name and alias matching is **case-insensitive**.
    #[must_use]
    pub fn find(&self, input: &str) -> Option<(Arc<dyn Command>, String)> {
        let input = input.trim();
        if !input.starts_with('/') {
            return None;
        }

        let without_slash = &input[1..];
        let (cmd_name, args) = match without_slash.split_once(char::is_whitespace) {
            Some((name, rest)) => (name, rest.trim().to_owned()),
            None => (without_slash, String::new()),
        };

        let cmd_lower = cmd_name.to_ascii_lowercase();

        self.commands
            .iter()
            .find(|c| {
                c.name().eq_ignore_ascii_case(&cmd_lower)
                    || c.aliases().iter().any(|a| a.eq_ignore_ascii_case(&cmd_lower))
            })
            .map(|c| (Arc::clone(c), args))
    }

    /// All registered commands (including hidden ones).
    #[must_use]
    pub fn all(&self) -> &[Arc<dyn Command>] {
        &self.commands
    }

    /// Only commands whose `is_hidden()` returns `false`.
    #[must_use]
    pub fn visible(&self) -> Vec<Arc<dyn Command>> {
        self.commands
            .iter()
            .filter(|c| !c.is_hidden())
            .cloned()
            .collect()
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::{CommandContext, CommandResult};
    use async_trait::async_trait;

    // --- helpers -------------------------------------------------------

    struct Dummy {
        n: &'static str,
        a: &'static [&'static str],
        hidden: bool,
    }

    impl Dummy {
        fn visible(name: &'static str, aliases: &'static [&'static str]) -> Self {
            Self { n: name, a: aliases, hidden: false }
        }
        fn hidden(name: &'static str) -> Self {
            Self { n: name, a: &[], hidden: true }
        }
    }

    #[async_trait]
    impl Command for Dummy {
        fn name(&self) -> &str { self.n }
        fn aliases(&self) -> &[&str] { self.a }
        fn description(&self) -> &str { "dummy" }
        fn is_hidden(&self) -> bool { self.hidden }
        async fn execute(&self, _args: &str, _ctx: &mut CommandContext) -> anyhow::Result<CommandResult> {
            Ok(CommandResult::Handled(Some(self.n.to_owned())))
        }
    }

    fn registry_with_dummy() -> CommandRegistry {
        let mut r = CommandRegistry::new();
        r.register(Arc::new(Dummy::visible("help", &["h", "?"])));
        r.register(Arc::new(Dummy::hidden("debug")));
        r
    }

    // --- tests ---------------------------------------------------------

    #[test]
    fn find_by_name() {
        let r = registry_with_dummy();
        let (cmd, args) = r.find("/help").expect("should find help");
        assert_eq!(cmd.name(), "help");
        assert!(args.is_empty());
    }

    #[test]
    fn find_by_alias() {
        let r = registry_with_dummy();
        assert!(r.find("/h").is_some());
        assert!(r.find("/?").is_some());
    }

    #[test]
    fn find_no_slash_returns_none() {
        let r = registry_with_dummy();
        assert!(r.find("help").is_none());
        assert!(r.find("").is_none());
    }

    #[test]
    fn find_unregistered_returns_none() {
        let r = registry_with_dummy();
        assert!(r.find("/nonexistent").is_none());
    }

    #[test]
    fn find_case_insensitive() {
        let r = registry_with_dummy();
        assert!(r.find("/HELP").is_some());
        assert!(r.find("/Help").is_some());
        assert!(r.find("/H").is_some());
    }

    #[test]
    fn visible_filters_hidden() {
        let r = registry_with_dummy();
        let vis = r.visible();
        assert_eq!(vis.len(), 1);
        assert_eq!(vis[0].name(), "help");
    }

    #[test]
    fn find_extracts_args() {
        let r = registry_with_dummy();
        let (_, args) = r.find("/help version").unwrap();
        assert_eq!(args, "version");
    }

    #[test]
    fn find_slash_only_returns_none() {
        let r = registry_with_dummy();
        assert!(r.find("/").is_none());
    }

    #[test]
    fn find_very_long_command_name_returns_none() {
        let r = registry_with_dummy();
        let long = format!("/{}", "x".repeat(1000));
        assert!(r.find(&long).is_none());
    }
}
