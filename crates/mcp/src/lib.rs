//! Claude Code — MCP (Model Context Protocol) client.

pub mod client;
pub mod manager;
pub mod transport;
pub mod types;

pub use client::McpClient;
pub use manager::McpConnectionManager;
pub use types::{McpResourceInfo, McpToolInfo};
