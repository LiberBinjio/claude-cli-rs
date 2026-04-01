//! MCP connection manager — manages multiple MCP server connections.

use std::collections::HashMap;

use tracing::{info, warn};

use crate::client::McpClient;
use crate::types::{McpServerConfig, McpToolInfo};

/// Manages the lifecycle of multiple MCP server connections.
pub struct McpConnectionManager {
    clients: HashMap<String, McpClient>,
}

impl McpConnectionManager {
    /// Create an empty manager with no connections.
    #[must_use]
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
        }
    }

    /// Connect to all MCP servers declared in the given config map.
    ///
    /// Servers that fail to connect are logged as warnings but do not abort the
    /// entire operation.
    pub async fn connect_from_config(
        &mut self,
        servers: &HashMap<String, McpServerConfig>,
    ) -> anyhow::Result<()> {
        for (name, cfg) in servers {
            if let Err(e) = self.connect_server(name, &cfg.command, &cfg.args, &cfg.env).await {
                warn!(server = %name, error = %e, "failed to connect MCP server");
            }
        }
        Ok(())
    }

    /// Connect to a single MCP server, run `initialize`, and store the client.
    pub async fn connect_server(
        &mut self,
        name: &str,
        command: &str,
        args: &[String],
        env: &HashMap<String, String>,
    ) -> anyhow::Result<()> {
        let mut client = McpClient::connect(command, args, env).await?;
        client.initialize().await?;
        info!(server = %name, "MCP server connected and initialized");
        self.clients.insert(name.to_owned(), client);
        Ok(())
    }

    /// Get a mutable reference to a connected client by server name.
    #[must_use]
    pub fn get(&mut self, name: &str) -> Option<&mut McpClient> {
        self.clients.get_mut(name)
    }

    /// Collect tools from all connected servers, prefixed with server name.
    pub async fn all_tools(&mut self) -> Vec<(String, McpToolInfo)> {
        let mut result = Vec::new();
        // We need to iterate mutably, so collect keys first.
        let keys: Vec<String> = self.clients.keys().cloned().collect();
        for key in keys {
            if let Some(client) = self.clients.get_mut(&key) {
                match client.list_tools().await {
                    Ok(tools) => {
                        for tool in tools {
                            result.push((key.clone(), tool));
                        }
                    }
                    Err(e) => {
                        warn!(server = %key, error = %e, "failed to list tools");
                    }
                }
            }
        }
        result
    }

    /// Close all connections.
    pub async fn close_all(self) -> anyhow::Result<()> {
        for (name, client) in self.clients {
            if let Err(e) = client.close().await {
                warn!(server = %name, error = %e, "error closing MCP connection");
            }
        }
        Ok(())
    }
}

impl Default for McpConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_manager_is_empty() {
        let mgr = McpConnectionManager::new();
        assert!(mgr.clients.is_empty());
    }

    #[test]
    fn get_nonexistent_returns_none() {
        let mut mgr = McpConnectionManager::new();
        assert!(mgr.get("nope").is_none());
    }

    #[test]
    fn default_is_empty() {
        let mgr = McpConnectionManager::default();
        assert!(mgr.clients.is_empty());
    }
}
