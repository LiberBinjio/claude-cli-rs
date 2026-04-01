//! MCP client — wraps a [`StdioTransport`] with the MCP protocol handshake and tool/resource RPCs.

use std::collections::HashMap;

use tracing::{debug, info};

use crate::transport::StdioTransport;
use crate::types::{
    JsonRpcNotification, JsonRpcRequest, JsonRpcResponse, McpResourceInfo, McpToolInfo,
    ServerCapabilities,
};

/// High-level MCP client.
pub struct McpClient {
    transport: StdioTransport,
    next_id: u64,
    /// Server capabilities received during `initialize`.
    pub server_info: Option<ServerCapabilities>,
}

impl McpClient {
    /// Spawn and connect to an MCP server, but do **not** initialize yet.
    pub async fn connect(
        command: &str,
        args: &[String],
        env: &HashMap<String, String>,
    ) -> anyhow::Result<Self> {
        let transport = StdioTransport::spawn(command, args, env).await?;
        Ok(Self {
            transport,
            next_id: 1,
            server_info: None,
        })
    }

    /// Perform the MCP `initialize` handshake.
    pub async fn initialize(&mut self) -> anyhow::Result<()> {
        let params = serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "claude-cli-rs",
                "version": env!("CARGO_PKG_VERSION")
            }
        });

        let result = self.call("initialize", Some(params)).await?;

        let caps: ServerCapabilities =
            serde_json::from_value(result.get("capabilities").cloned().unwrap_or_default())
                .unwrap_or_default();
        self.server_info = Some(caps);

        // Send initialized notification
        let notif = JsonRpcNotification::new("notifications/initialized", None);
        self.transport.send_notification(&notif).await?;

        info!("MCP initialization complete");
        Ok(())
    }

    /// Retrieve the list of tools the server exposes.
    pub async fn list_tools(&mut self) -> anyhow::Result<Vec<McpToolInfo>> {
        let result = self.call("tools/list", None).await?;
        let tools: Vec<McpToolInfo> = serde_json::from_value(
            result
                .get("tools")
                .cloned()
                .unwrap_or(serde_json::Value::Array(vec![])),
        )?;
        debug!(count = tools.len(), "listed MCP tools");
        Ok(tools)
    }

    /// Invoke a tool on the MCP server.
    pub async fn call_tool(
        &mut self,
        name: &str,
        arguments: serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let params = serde_json::json!({
            "name": name,
            "arguments": arguments,
        });
        let result = self.call("tools/call", Some(params)).await?;
        Ok(result)
    }

    /// Retrieve the list of resources the server exposes.
    pub async fn list_resources(&mut self) -> anyhow::Result<Vec<McpResourceInfo>> {
        let result = self.call("resources/list", None).await?;
        let resources: Vec<McpResourceInfo> = serde_json::from_value(
            result
                .get("resources")
                .cloned()
                .unwrap_or(serde_json::Value::Array(vec![])),
        )?;
        debug!(count = resources.len(), "listed MCP resources");
        Ok(resources)
    }

    /// Read a resource by URI.
    pub async fn read_resource(&mut self, uri: &str) -> anyhow::Result<String> {
        let params = serde_json::json!({ "uri": uri });
        let result = self.call("resources/read", Some(params)).await?;
        // Extract text from the first content item
        let contents = result
            .get("contents")
            .and_then(|c| c.as_array())
            .cloned()
            .unwrap_or_default();
        let text = contents
            .first()
            .and_then(|c| c.get("text"))
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_owned();
        Ok(text)
    }

    /// Shut down the transport.
    pub async fn close(self) -> anyhow::Result<()> {
        self.transport.close().await
    }

    /// Low-level RPC helper: send request, receive response, extract `.result`.
    async fn call(
        &mut self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> anyhow::Result<serde_json::Value> {
        let id = self.next_id;
        self.next_id += 1;

        let request = JsonRpcRequest::new(id, method, params);
        self.transport.send_request(&request).await?;

        let response: JsonRpcResponse = self.transport.receive().await?;

        if let Some(err) = response.error {
            anyhow::bail!("MCP error {}: {}", err.code, err.message);
        }

        Ok(response.result.unwrap_or(serde_json::Value::Null))
    }
}
