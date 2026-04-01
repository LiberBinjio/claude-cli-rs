//! WebSearchTool: search the web using DuckDuckGo HTML endpoint.

use claude_core::tool::{PermissionCheck, Tool, ToolInputSchema, ToolResult, ToolUseContext};
use async_trait::async_trait;
use serde_json::Value;

const MAX_SEARCH_RESULT_CHARS: usize = 5000;

/// Tool for searching the web.
pub struct WebSearchTool;

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &str { "WebSearch" }

    fn description(&self) -> &str {
        "Search the web for information using DuckDuckGo."
    }

    fn input_schema(&self) -> ToolInputSchema {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query"
                }
            },
            "required": ["query"]
        })
    }

    fn is_read_only(&self, _input: &Value) -> bool { true }

    fn needs_permission(&self, _input: &Value) -> PermissionCheck {
        PermissionCheck::Allowed
    }

    async fn call(&self, input: Value, _ctx: &mut ToolUseContext) -> anyhow::Result<ToolResult> {
        let query = input
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing 'query' parameter"))?;

        let encoded = urlencoding::encode(query);
        let url = format!("https://html.duckduckgo.com/html/?q={encoded}");

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .user_agent("claude-cli-rs/0.1")
            .build()?;

        let resp = match client.get(&url).send().await {
            Ok(r) => r,
            Err(e) => return Ok(ToolResult::error(format!("Search request failed: {e}"))),
        };

        let body = resp.text().await?;
        let text = crate::web_fetch::strip_html_tags(&body);

        let truncated = if text.len() > MAX_SEARCH_RESULT_CHARS {
            &text[..MAX_SEARCH_RESULT_CHARS]
        } else {
            &text
        };

        Ok(ToolResult::text(truncated.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema() {
        let tool = WebSearchTool;
        assert_eq!(tool.name(), "WebSearch");
        let schema = tool.input_schema();
        assert_eq!(schema["required"][0], "query");
    }

    #[test]
    fn test_read_only() {
        let tool = WebSearchTool;
        assert!(tool.is_read_only(&serde_json::json!({})));
    }
}
