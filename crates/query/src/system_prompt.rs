//! System prompt builder.

use claude_core::config::AppConfig;
use std::path::Path;

/// Build the complete system prompt for the API.
///
/// Includes role definition, working directory, platform info,
/// available tools, and any user-provided instructions.
#[must_use]
pub fn build_system_prompt(config: &AppConfig, cwd: &Path, tool_names: &[String]) -> String {
    let mut prompt = String::with_capacity(4096);

    // Role definition
    prompt.push_str(
        "You are Claude, a helpful AI assistant powered by Anthropic's Claude model.\n\
         You are operating as a terminal-based coding assistant.\n\n",
    );

    // Working directory
    prompt.push_str(&format!(
        "Current working directory: {}\n\n",
        cwd.display()
    ));

    // Platform info
    prompt.push_str(&format!("Platform: {}\n", std::env::consts::OS));
    prompt.push_str(&format!("Architecture: {}\n\n", std::env::consts::ARCH));

    // Available tools
    if !tool_names.is_empty() {
        prompt.push_str("Available tools:\n");
        for name in tool_names {
            prompt.push_str(&format!("- {name}\n"));
        }
        prompt.push('\n');
    }

    // User custom instructions
    if let Some(ref instructions) = config.custom_system_prompt {
        prompt.push_str("User instructions:\n");
        prompt.push_str(instructions);
        prompt.push('\n');
    }

    prompt
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_basic_prompt() {
        let config = AppConfig::default();
        let cwd = PathBuf::from("/tmp/test");
        let tools = vec!["Bash".to_string(), "FileEdit".to_string()];
        let prompt = build_system_prompt(&config, &cwd, &tools);
        assert!(prompt.contains("Claude"));
        assert!(prompt.contains("Bash"));
        assert!(prompt.contains("FileEdit"));
    }

    #[test]
    fn test_prompt_with_custom_instructions() {
        let mut config = AppConfig::default();
        config.custom_system_prompt = Some("Always use Rust.".to_string());
        let prompt = build_system_prompt(&config, &PathBuf::from("."), &[]);
        assert!(prompt.contains("Always use Rust."));
    }

    #[test]
    fn test_prompt_contains_platform() {
        let config = AppConfig::default();
        let prompt = build_system_prompt(&config, &PathBuf::from("."), &[]);
        assert!(prompt.contains("Platform:"));
        assert!(prompt.contains("Architecture:"));
    }

    #[test]
    fn test_prompt_empty_tools() {
        let config = AppConfig::default();
        let prompt = build_system_prompt(&config, &PathBuf::from("."), &[]);
        assert!(!prompt.contains("Available tools"));
    }
}
