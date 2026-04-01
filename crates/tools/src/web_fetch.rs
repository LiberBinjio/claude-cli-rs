//! WebFetchTool: fetch web page contents with SSRF protection.

use claude_core::tool::{PermissionCheck, Tool, ToolInputSchema, ToolResult, ToolUseContext};
use async_trait::async_trait;
use serde_json::Value;
use std::net::IpAddr;

const MAX_RESPONSE_SIZE: usize = 500_000;

/// Tool for fetching web page contents.
pub struct WebFetchTool;

#[async_trait]
impl Tool for WebFetchTool {
    fn name(&self) -> &str { "WebFetch" }

    fn description(&self) -> &str {
        "Fetch the contents of a web page and return as text."
    }

    fn input_schema(&self) -> ToolInputSchema {
        serde_json::json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The URL to fetch (http or https only)"
                },
                "prompt": {
                    "type": "string",
                    "description": "Optional: what to extract from the page"
                }
            },
            "required": ["url"]
        })
    }

    fn is_read_only(&self, _input: &Value) -> bool { true }

    fn needs_permission(&self, _input: &Value) -> PermissionCheck {
        PermissionCheck::Allowed
    }

    async fn call(&self, input: Value, _ctx: &mut ToolUseContext) -> anyhow::Result<ToolResult> {
        let url = input
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing 'url' parameter"))?;

        // SSRF protection: only http/https schemes
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Ok(ToolResult::error(
                "Only http:// and https:// URLs are supported",
            ));
        }

        // SSRF protection: block private/reserved IPs
        if let Some(host) = extract_host(url) {
            if is_private_host(&host) {
                return Ok(ToolResult::error(
                    "Requests to private/reserved IP addresses are blocked",
                ));
            }
        }

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("claude-cli-rs/0.1")
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()?;

        let response = match client.get(url).send().await {
            Ok(r) => r,
            Err(e) => return Ok(ToolResult::error(format!("Request failed: {e}"))),
        };

        let status = response.status();
        if !status.is_success() {
            return Ok(ToolResult::error(format!(
                "HTTP {}: {}",
                status.as_u16(),
                status.canonical_reason().unwrap_or("Unknown")
            )));
        }

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let body = response.text().await?;

        // Convert HTML to plain text
        let text = if content_type.contains("html") {
            strip_html_tags(&body)
        } else {
            body
        };

        // Truncate
        let text = if text.len() > MAX_RESPONSE_SIZE {
            format!(
                "{}...\n[Truncated: content exceeded {MAX_RESPONSE_SIZE} characters]",
                &text[..MAX_RESPONSE_SIZE]
            )
        } else {
            text
        };

        Ok(ToolResult::text(text))
    }
}

/// Extract hostname from a URL string.
fn extract_host(url: &str) -> Option<String> {
    // Strip scheme
    let after_scheme = url.split("://").nth(1)?;
    // Take until path/port
    let host = after_scheme.split('/').next()?;
    let host = host.split(':').next()?;
    Some(host.to_lowercase())
}

/// Check if a hostname resolves to a private/reserved IP range.
fn is_private_host(host: &str) -> bool {
    // Check common private hostnames
    if host == "localhost" || host.ends_with(".local") || host.ends_with(".internal") {
        return true;
    }
    // Try parsing as IP directly
    if let Ok(ip) = host.parse::<IpAddr>() {
        return is_private_ip(ip);
    }
    false
}

/// Check if an IP address is in a private/reserved range.
fn is_private_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let octets = v4.octets();
            v4.is_loopback()                          // 127.0.0.0/8
                || octets[0] == 10                     // 10.0.0.0/8
                || (octets[0] == 172 && (16..=31).contains(&octets[1])) // 172.16-31.0.0/12
                || (octets[0] == 192 && octets[1] == 168) // 192.168.0.0/16
                || (octets[0] == 169 && octets[1] == 254) // 169.254.0.0/16 link-local
                || v4.is_unspecified()                 // 0.0.0.0
        }
        IpAddr::V6(v6) => v6.is_loopback() || v6.is_unspecified(),
    }
}

/// Strip HTML tags, script/style content, and normalize whitespace.
pub(crate) fn strip_html_tags(html: &str) -> String {
    let mut result = String::with_capacity(html.len() / 2);
    let mut in_tag = false;
    let mut in_script = false;
    let mut in_style = false;
    let lower = html.to_lowercase();
    let chars: Vec<char> = html.chars().collect();

    let mut i = 0;
    while i < chars.len() {
        if !in_tag && lower[i..].starts_with("<script") {
            in_script = true;
            in_tag = true;
        } else if in_script && lower[i..].starts_with("</script") {
            in_script = false;
            in_tag = true;
        } else if !in_tag && lower[i..].starts_with("<style") {
            in_style = true;
            in_tag = true;
        } else if in_style && lower[i..].starts_with("</style") {
            in_style = false;
            in_tag = true;
        } else if chars[i] == '<' {
            in_tag = true;
        } else if chars[i] == '>' {
            in_tag = false;
            // Add space to separate neighboring text blocks
            if !result.ends_with(' ') && !result.ends_with('\n') {
                result.push(' ');
            }
            i += 1;
            continue;
        } else if !in_tag && !in_script && !in_style {
            result.push(chars[i]);
        }
        i += 1;
    }

    // Compress consecutive whitespace
    let mut cleaned = String::new();
    let mut last_ws = false;
    for c in result.chars() {
        if c.is_whitespace() {
            if !last_ws {
                cleaned.push(' ');
            }
            last_ws = true;
        } else {
            cleaned.push(c);
            last_ws = false;
        }
    }
    cleaned.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_html_basic() {
        assert_eq!(strip_html_tags("<p>Hello <b>world</b></p>"), "Hello world");
    }

    #[test]
    fn test_strip_html_script() {
        let html = "<p>Hi</p><script>alert(1)</script><p>Bye</p>";
        let text = strip_html_tags(html);
        assert!(text.contains("Hi"));
        assert!(text.contains("Bye"));
        assert!(!text.contains("alert"));
    }

    #[test]
    fn test_strip_html_style() {
        let html = "<style>.x{color:red}</style><p>Content</p>";
        let text = strip_html_tags(html);
        assert!(text.contains("Content"));
        assert!(!text.contains("color"));
    }

    #[test]
    fn test_private_ip() {
        assert!(is_private_ip("127.0.0.1".parse().unwrap()));
        assert!(is_private_ip("10.0.0.1".parse().unwrap()));
        assert!(is_private_ip("172.16.0.1".parse().unwrap()));
        assert!(is_private_ip("192.168.1.1".parse().unwrap()));
        assert!(!is_private_ip("8.8.8.8".parse().unwrap()));
        assert!(!is_private_ip("1.1.1.1".parse().unwrap()));
    }

    #[test]
    fn test_private_host() {
        assert!(is_private_host("localhost"));
        assert!(is_private_host("127.0.0.1"));
        assert!(is_private_host("10.0.0.5"));
        assert!(!is_private_host("example.com"));
    }

    #[test]
    fn test_extract_host() {
        assert_eq!(extract_host("https://example.com/path"), Some("example.com".into()));
        assert_eq!(extract_host("http://localhost:8080"), Some("localhost".into()));
    }

    #[test]
    fn test_schema() {
        let tool = WebFetchTool;
        assert_eq!(tool.name(), "WebFetch");
        assert_eq!(tool.input_schema()["required"][0], "url");
    }
}
