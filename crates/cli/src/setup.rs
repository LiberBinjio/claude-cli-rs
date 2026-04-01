//! Application services initialization.

use std::path::PathBuf;
use std::sync::Arc;

use claude_api::ApiClient;
use claude_auth::providers::resolve_api_provider;
use claude_commands::CommandRegistry;
use claude_core::config::AppConfig;
use claude_query::engine::QueryEngine;
use claude_query::system_prompt::build_system_prompt;
use claude_query::ToolSet;

use crate::args::CliArgs;

/// Initialized application services, ready for the main loop.
pub struct AppServices {
    /// The query engine that drives conversations.
    pub engine: QueryEngine,
}

/// Build all services from CLI arguments.
///
/// This performs authentication, creates the API client, tool & command
/// registries, and wires everything into a [`QueryEngine`].
pub fn setup(args: &CliArgs) -> anyhow::Result<AppServices> {
    // 1. Working directory
    let cwd = match &args.cwd {
        Some(dir) => PathBuf::from(dir),
        None => std::env::current_dir()?,
    };

    // 2. Configuration
    let config = AppConfig {
        model: args.model.clone(),
        ..AppConfig::default()
    };

    // 3. Authentication
    let provider = resolve_api_provider().map_err(|e| {
        anyhow::anyhow!(
            "Authentication failed: {e}.\n\
             Set ANTHROPIC_API_KEY or run `claude login`."
        )
    })?;

    // 4. API client
    let api_client = Arc::new(ApiClient::new(provider, config.clone()));

    // 5. Tools (empty for now — will be populated by claude_tools)
    let tool_set = Arc::new(ToolSet::new());

    // 6. Commands
    let cmd_registry = Arc::new(CommandRegistry::new());

    // 7. Query engine
    let tool_names = tool_set.names();
    let system_prompt = build_system_prompt(&config, &cwd, &tool_names);

    let mut engine = QueryEngine::new(api_client, tool_set, cmd_registry, cwd);
    engine.set_model(&config.model);
    engine.set_system_prompt(system_prompt);

    Ok(AppServices { engine })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_setup_fails_without_api_key() {
        // With no API key set, setup should return an auth error.
        let args = CliArgs {
            prompt: None,
            print: false,
            model: "test-model".into(),
            cwd: Some(".".into()),
            resume: None,
            verbose: false,
            command: None,
        };
        let result = setup(&args);
        // Should fail on authentication (no ANTHROPIC_API_KEY set in test env)
        assert!(result.is_err());
    }
}