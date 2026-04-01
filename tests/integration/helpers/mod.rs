#![allow(dead_code)]

pub mod mock_api;

use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::RwLock;

use claude_api::ApiClient;
use claude_auth::ApiProvider;
use claude_commands::builtin::register_builtins;
use claude_commands::CommandRegistry;
use claude_core::config::AppConfig;
use claude_core::state::AppState;
use claude_core::tool::{ToolUseContext};
use claude_query::QueryEngine;
use claude_tools::ToolRegistry;

use mock_api::MockAnthropicApi;

/// Create a fully wired [`QueryEngine`] pointing at the mock server.
pub async fn create_test_engine(mock: &MockAnthropicApi, cwd: PathBuf) -> QueryEngine {
    let provider = ApiProvider::Anthropic {
        api_key: "test-key".into(),
        base_url: Some(mock.base_url.clone()),
    };
    let config = AppConfig::default();
    let api_client = Arc::new(ApiClient::new(provider, config.clone()));

    let mut tool_registry = ToolRegistry::new();
    claude_tools::register_p0_tools(&mut tool_registry);
    claude_tools::register_p0b_tools(&mut tool_registry);
    let tool_registry = Arc::new(tool_registry);

    let mut cmd_registry = CommandRegistry::new();
    register_builtins(&mut cmd_registry);
    let cmd_registry = Arc::new(cmd_registry);

    let app_state = Arc::new(RwLock::new(AppState::new(cwd.clone(), config)));

    QueryEngine::new(api_client, tool_registry, cmd_registry, app_state, cwd)
}

/// Create a [`ToolUseContext`] for testing tools against a temporary directory.
pub fn create_tool_context(cwd: &std::path::Path) -> ToolUseContext {
    ToolUseContext {
        cwd: cwd.to_path_buf(),
        permission_mode: claude_core::permission::PermissionMode::FullAuto,
        tool_use_id: "test-tool-use-id".to_string(),
        session_id: "test-session".to_string(),
    }
}