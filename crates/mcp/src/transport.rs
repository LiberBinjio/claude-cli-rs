//! stdio transport — spawn an MCP server subprocess and communicate via JSON-RPC over stdin/stdout.

use std::collections::HashMap;
use std::process::Stdio;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tracing::debug;

use crate::types::{JsonRpcNotification, JsonRpcRequest, JsonRpcResponse};

/// A stdio-based transport for MCP servers.
///
/// Launches a child process and communicates through newline-delimited JSON on
/// stdin (outgoing) / stdout (incoming).
pub struct StdioTransport {
    child: Child,
    stdin: ChildStdin,
    reader: BufReader<ChildStdout>,
}

impl StdioTransport {
    /// Spawn an MCP server process and prepare the transport.
    pub async fn spawn(
        command: &str,
        args: &[String],
        env: &HashMap<String, String>,
    ) -> anyhow::Result<Self> {
        let mut child = Command::new(command)
            .args(args)
            .envs(env)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("failed to open child stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("failed to open child stdout"))?;

        debug!(command, "MCP server process spawned");

        Ok(Self {
            child,
            stdin,
            reader: BufReader::new(stdout),
        })
    }

    /// Send a JSON-RPC request, appending a newline.
    pub async fn send_request(&mut self, request: &JsonRpcRequest) -> anyhow::Result<()> {
        let mut payload = serde_json::to_string(request)?;
        payload.push('\n');
        self.stdin.write_all(payload.as_bytes()).await?;
        self.stdin.flush().await?;
        debug!(method = %request.method, id = request.id, "sent request");
        Ok(())
    }

    /// Send a JSON-RPC notification (no id, no response expected), appending a newline.
    pub async fn send_notification(
        &mut self,
        notification: &JsonRpcNotification,
    ) -> anyhow::Result<()> {
        let mut payload = serde_json::to_string(notification)?;
        payload.push('\n');
        self.stdin.write_all(payload.as_bytes()).await?;
        self.stdin.flush().await?;
        debug!(method = %notification.method, "sent notification");
        Ok(())
    }

    /// Read a single line from stdout and deserialize it as a JSON-RPC response.
    ///
    /// Applies a 30-second timeout to avoid hanging forever.
    pub async fn receive(&mut self) -> anyhow::Result<JsonRpcResponse> {
        let mut line = String::new();
        let read_result = tokio::time::timeout(
            std::time::Duration::from_secs(30),
            self.reader.read_line(&mut line),
        )
        .await;

        match read_result {
            Ok(Ok(0)) => anyhow::bail!("MCP server closed stdout"),
            Ok(Ok(_)) => {
                let resp: JsonRpcResponse = serde_json::from_str(line.trim())?;
                Ok(resp)
            }
            Ok(Err(e)) => Err(e.into()),
            Err(_) => anyhow::bail!("timed out waiting for MCP server response (30s)"),
        }
    }

    /// Close stdin and wait for the child process to exit.
    pub async fn close(mut self) -> anyhow::Result<()> {
        drop(self.stdin);
        let _ = self.child.wait().await;
        debug!("MCP server process closed");
        Ok(())
    }
}
