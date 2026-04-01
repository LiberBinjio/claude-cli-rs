//! `/memory` — show CLAUDE.md memory file status.

use async_trait::async_trait;

use crate::command::{Command, CommandContext, CommandResult};

/// Display CLAUDE.md memory file location and status.
pub struct MemoryCommand;

#[async_trait]
impl Command for MemoryCommand {
    fn name(&self) -> &str {
        "memory"
    }

    fn description(&self) -> &str {
        "Show CLAUDE.md memory status"
    }

    async fn execute(
        &self,
        _args: &str,
        _ctx: &mut CommandContext,
    ) -> anyhow::Result<CommandResult> {
        let home = home_dir();
        let global_memory = home.join(".claude").join("CLAUDE.md");
        let local_memory = std::path::Path::new("CLAUDE.md");

        let mut lines = Vec::new();
        lines.push("Memory Files".to_string());

        if global_memory.exists() {
            let size = std::fs::metadata(&global_memory)
                .map(|m| m.len())
                .unwrap_or(0);
            lines.push(format!(
                "  Global: {} ({} bytes)",
                global_memory.display(),
                size
            ));
        } else {
            lines.push(format!(
                "  Global: {} (not found)",
                global_memory.display()
            ));
        }

        if local_memory.exists() {
            let size = std::fs::metadata(local_memory)
                .map(|m| m.len())
                .unwrap_or(0);
            lines.push(format!("  Local:  ./CLAUDE.md ({size} bytes)"));
        } else {
            lines.push("  Local:  ./CLAUDE.md (not found)".to_string());
        }

        Ok(CommandResult::Handled(Some(lines.join("\n"))))
    }
}

/// Platform-specific home directory lookup.
fn home_dir() -> std::path::PathBuf {
    #[cfg(windows)]
    {
        std::env::var("USERPROFILE")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::path::PathBuf::from("C:\\Users\\Default"))
    }
    #[cfg(not(windows))]
    {
        std::env::var("HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::path::PathBuf::from("/tmp"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_output() {
        let cmd = MemoryCommand;
        let mut ctx = CommandContext {
            placeholder_state: (),
            event_tx: None,
        };
        let result = cmd.execute("", &mut ctx).await.unwrap();
        match result {
            CommandResult::Handled(Some(msg)) => {
                assert!(msg.contains("Memory Files"));
                assert!(msg.contains("Global:"));
            }
            _ => panic!("expected memory status"),
        }
    }
}
