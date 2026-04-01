//! Application configuration types.

use crate::permission::{PermissionMode, PermissionRule};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Top-level application configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Primary model to use for queries.
    pub model: String,
    /// Smaller/faster model for auxiliary tasks.
    pub small_fast_model: String,
    /// Active permission mode.
    pub permission_mode: PermissionMode,
    /// API key (if using direct key auth).
    pub api_key: Option<String>,
    /// Custom API base URL.
    pub custom_api_url: Option<String>,
    /// Maximum total cost in USD before stopping.
    pub max_cost_usd: Option<f64>,
    /// Custom system prompt to prepend.
    pub custom_system_prompt: Option<String>,
    /// Explicit permission rules.
    #[serde(default)]
    pub permission_rules: Vec<PermissionRule>,
    /// MCP server configurations.
    #[serde(default)]
    pub mcp_servers: HashMap<String, McpServerConfig>,
    /// Additional environment variables.
    #[serde(default)]
    pub env: HashMap<String, String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            model: "claude-sonnet-4-20250514".into(),
            small_fast_model: "claude-haiku-4-20250414".into(),
            permission_mode: PermissionMode::Default,
            api_key: None,
            custom_api_url: None,
            max_cost_usd: None,
            custom_system_prompt: None,
            permission_rules: Vec::new(),
            mcp_servers: HashMap::new(),
            env: HashMap::new(),
        }
    }
}

/// Configuration for an MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Command to launch the server.
    pub command: String,
    /// Arguments to the command.
    #[serde(default)]
    pub args: Vec<String>,
    /// Environment variables for the server process.
    #[serde(default)]
    pub env: HashMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_config_defaults() {
        let config = AppConfig::default();
        assert_eq!(config.model, "claude-sonnet-4-20250514");
        assert_eq!(config.small_fast_model, "claude-haiku-4-20250414");
        assert_eq!(config.permission_mode, PermissionMode::Default);
        assert!(config.api_key.is_none());
        assert!(config.custom_api_url.is_none());
        assert!(config.max_cost_usd.is_none());
        assert!(config.custom_system_prompt.is_none());
        assert!(config.permission_rules.is_empty());
        assert!(config.mcp_servers.is_empty());
        assert!(config.env.is_empty());
    }

    #[test]
    fn test_app_config_serde_roundtrip() {
        let config = AppConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let parsed: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.model, config.model);
        assert_eq!(parsed.small_fast_model, config.small_fast_model);
    }

    #[test]
    fn test_mcp_server_config_serde() {
        let cfg = McpServerConfig {
            command: "npx".into(),
            args: vec!["-y".into(), "mcp-server".into()],
            env: HashMap::new(),
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let parsed: McpServerConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.command, "npx");
        assert_eq!(parsed.args.len(), 2);
    }

    #[test]
    fn test_app_config_with_mcp_servers() {
        let mut config = AppConfig::default();
        config.mcp_servers.insert(
            "test".into(),
            McpServerConfig {
                command: "node".into(),
                args: vec!["server.js".into()],
                env: HashMap::new(),
            },
        );
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("test"));
        assert!(json.contains("node"));
    }
}
