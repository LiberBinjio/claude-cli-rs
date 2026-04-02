//! Copilot proxy utilities: health check and connection validation.

use tracing::debug;

/// Default Agent Maestro proxy URL.
pub const DEFAULT_PROXY_URL: &str = "http://127.0.0.1:23333/api/anthropic";

/// Check if the Agent Maestro proxy is reachable.
///
/// Sends a GET request to the `/api/v1/info` endpoint and returns `true`
/// if it responds with a 2xx status code.
pub async fn check_proxy_health(proxy_url: &str) -> bool {
    // Extract base URL (strip /api/anthropic suffix if present)
    let base = proxy_url
        .trim_end_matches('/')
        .trim_end_matches("/api/anthropic")
        .trim_end_matches('/');
    let info_url = format!("{base}/api/v1/info");

    debug!("Checking Agent Maestro proxy health: {info_url}");

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build();

    let Ok(client) = client else {
        return false;
    };

    match client.get(&info_url).send().await {
        Ok(resp) => {
            let healthy = resp.status().is_success();
            debug!(
                "Proxy health check: status={}, healthy={healthy}",
                resp.status()
            );
            healthy
        }
        Err(e) => {
            debug!("Proxy health check failed: {e}");
            false
        }
    }
}

/// Instructions to show when the proxy is not reachable.
pub const PROXY_SETUP_INSTRUCTIONS: &str = "\
Agent Maestro proxy not detected on localhost:23333.

To use GitHub Copilot authentication:
  1. Install the \"Agent Maestro\" extension in VS Code
  2. Open VS Code and start the proxy:
     Cmd+Shift+P (macOS) / Ctrl+Shift+P (Windows/Linux)
     -> \"Agent Maestro: Start API Server\"
  3. Restart claude-cli-rs: claude --copilot";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_proxy_url() {
        assert_eq!(DEFAULT_PROXY_URL, "http://127.0.0.1:23333/api/anthropic");
    }

    #[tokio::test]
    async fn test_proxy_health_unreachable() {
        // Should return false when nothing is listening on this port
        let healthy = check_proxy_health("http://127.0.0.1:19999/api/anthropic").await;
        assert!(!healthy);
    }

    #[test]
    fn test_proxy_setup_instructions_not_empty() {
        assert!(PROXY_SETUP_INSTRUCTIONS.contains("Agent Maestro"));
        assert!(PROXY_SETUP_INSTRUCTIONS.contains("--copilot"));
    }
}
