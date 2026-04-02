//! API provider routing: resolve which backend to use based on environment.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::debug;

/// The resolved API provider with credentials and endpoint info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApiProvider {
    /// Direct Anthropic API with an API key.
    Anthropic {
        api_key: String,
        base_url: String,
    },
    /// AWS Bedrock.
    Bedrock {
        region: String,
        profile: Option<String>,
    },
    /// Google Cloud Vertex AI.
    Vertex {
        project_id: String,
        region: String,
    },
    /// OAuth token-based access.
    OAuth {
        access_token: String,
        refresh_token: String,
    },
    /// GitHub Copilot via Agent Maestro local proxy.
    CopilotProxy {
        /// Proxy URL, e.g. "http://127.0.0.1:23333/api/anthropic"
        proxy_url: String,
        /// Optional API key for the proxy (if Agent Maestro has auth configured)
        api_key: Option<String>,
    },
}

/// Resolve the API provider from environment variables and stored credentials.
///
/// Priority order:
/// 1. `CLAUDE_CODE_USE_BEDROCK=1` -> Bedrock
/// 2. `CLAUDE_CODE_USE_VERTEX=1` -> Vertex
/// 3. `CLAUDE_CODE_USE_COPILOT=1` or `COPILOT_PROXY_URL` -> CopilotProxy
/// 4. Valid (non-expired) OAuth tokens -> OAuth
/// 5. API key (env / config file / keychain) -> Anthropic
/// 6. Error
pub fn resolve_api_provider() -> Result<ApiProvider> {
    // 1. Bedrock
    if std::env::var("CLAUDE_CODE_USE_BEDROCK").unwrap_or_default() == "1" {
        let region = std::env::var("AWS_REGION")
            .or_else(|_| std::env::var("AWS_DEFAULT_REGION"))
            .unwrap_or_else(|_| "us-east-1".into());
        let profile = std::env::var("AWS_PROFILE").ok();
        debug!("Resolved provider: Bedrock (region={region})");
        return Ok(ApiProvider::Bedrock { region, profile });
    }

    // 2. Vertex
    if std::env::var("CLAUDE_CODE_USE_VERTEX").unwrap_or_default() == "1" {
        let project_id = std::env::var("GOOGLE_CLOUD_PROJECT")
            .or_else(|_| std::env::var("GCLOUD_PROJECT"))
            .context(
                "CLAUDE_CODE_USE_VERTEX=1 but GOOGLE_CLOUD_PROJECT is not set",
            )?;
        let region = std::env::var("GOOGLE_CLOUD_REGION")
            .unwrap_or_else(|_| "us-central1".into());
        debug!("Resolved provider: Vertex (project={project_id}, region={region})");
        return Ok(ApiProvider::Vertex { project_id, region });
    }

    // 3. Copilot Proxy (Agent Maestro)
    if std::env::var("CLAUDE_CODE_USE_COPILOT").unwrap_or_default() == "1"
        || std::env::var("COPILOT_PROXY_URL").is_ok()
    {
        let proxy_url = std::env::var("COPILOT_PROXY_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:23333/api/anthropic".into());
        let api_key = std::env::var("COPILOT_PROXY_KEY").ok();
        debug!("Resolved provider: CopilotProxy (url={proxy_url})");
        return Ok(ApiProvider::CopilotProxy { proxy_url, api_key });
    }

    // 4. OAuth tokens (file or keychain)
    if let Some(tokens) = crate::oauth::load_tokens() {
        if !crate::oauth::is_token_expired(&tokens) {
            debug!("Resolved provider: OAuth (token from file)");
            return Ok(ApiProvider::OAuth {
                access_token: tokens.access_token,
                refresh_token: tokens.refresh_token,
            });
        }
    }
    if let Ok(Some(tokens)) = crate::keychain::load_oauth_tokens() {
        if !crate::oauth::is_token_expired(&tokens) {
            debug!("Resolved provider: OAuth (token from keychain)");
            return Ok(ApiProvider::OAuth {
                access_token: tokens.access_token,
                refresh_token: tokens.refresh_token,
            });
        }
    }

    // 5. API key
    if let Some(api_key) = crate::api_key::get_api_key() {
        let base_url = std::env::var("ANTHROPIC_BASE_URL")
            .unwrap_or_else(|_| "https://api.anthropic.com".into());
        debug!("Resolved provider: Anthropic (API key)");
        return Ok(ApiProvider::Anthropic { api_key, base_url });
    }

    // 6. Nothing found
    anyhow::bail!(
        "No API credentials found. Set ANTHROPIC_API_KEY, \
         run `claude auth login` for OAuth, or configure Bedrock/Vertex.\n\
         \n\
         Alternatively, use GitHub Copilot: claude --copilot\n\
         (requires VS Code + Agent Maestro extension)"
    )
}

/// Get the API base URL for a given provider.
#[must_use]
pub fn get_api_base_url(provider: &ApiProvider) -> String {
    match provider {
        ApiProvider::Anthropic { base_url, .. } => base_url.clone(),
        ApiProvider::Bedrock { region, .. } => {
            format!("https://bedrock-runtime.{region}.amazonaws.com")
        }
        ApiProvider::Vertex {
            project_id, region, ..
        } => format!(
            "https://{region}-aiplatform.googleapis.com/v1/projects/{project_id}/locations/{region}/publishers/anthropic/models"
        ),
        ApiProvider::OAuth { .. } => "https://api.anthropic.com".into(),
        ApiProvider::CopilotProxy { proxy_url, .. } => proxy_url.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    /// Guard that moves ~/.claude/oauth.json out of the way during tests
    /// and restores it when dropped.
    struct OAuthGuard {
        oauth_path: Option<std::path::PathBuf>,
        backup_path: Option<std::path::PathBuf>,
        moved: bool,
    }

    impl OAuthGuard {
        fn new() -> Self {
            let oauth_path = dirs::home_dir().map(|h| h.join(".claude").join("oauth.json"));
            let backup_path = dirs::home_dir().map(|h| h.join(".claude").join("oauth.json.test-bak"));
            let moved = match (&oauth_path, &backup_path) {
                (Some(op), Some(bp)) => std::fs::rename(op, bp).is_ok(),
                _ => false,
            };
            Self { oauth_path, backup_path, moved }
        }
    }

    impl Drop for OAuthGuard {
        fn drop(&mut self) {
            if self.moved {
                if let (Some(op), Some(bp)) = (&self.oauth_path, &self.backup_path) {
                    let _ = std::fs::rename(bp, op);
                }
            }
        }
    }

    #[test]
    #[serial]
    fn test_resolve_bedrock() {
        unsafe {
            std::env::set_var("CLAUDE_CODE_USE_BEDROCK", "1");
            std::env::set_var("AWS_REGION", "eu-west-1");
        }
        let provider = resolve_api_provider().unwrap();
        match provider {
            ApiProvider::Bedrock { region, .. } => assert_eq!(region, "eu-west-1"),
            other => panic!("Expected Bedrock, got {other:?}"),
        }
        unsafe {
            std::env::remove_var("CLAUDE_CODE_USE_BEDROCK");
            std::env::remove_var("AWS_REGION");
        }
    }

    #[test]
    #[serial]
    fn test_resolve_vertex() {
        unsafe {
            std::env::remove_var("CLAUDE_CODE_USE_BEDROCK");
            std::env::set_var("CLAUDE_CODE_USE_VERTEX", "1");
            std::env::set_var("GOOGLE_CLOUD_PROJECT", "my-project");
            std::env::set_var("GOOGLE_CLOUD_REGION", "asia-east1");
        }
        let provider = resolve_api_provider().unwrap();
        match provider {
            ApiProvider::Vertex { project_id, region } => {
                assert_eq!(project_id, "my-project");
                assert_eq!(region, "asia-east1");
            }
            other => panic!("Expected Vertex, got {other:?}"),
        }
        unsafe {
            std::env::remove_var("CLAUDE_CODE_USE_VERTEX");
            std::env::remove_var("GOOGLE_CLOUD_PROJECT");
            std::env::remove_var("GOOGLE_CLOUD_REGION");
        }
    }

    #[test]
    #[serial]
    fn test_resolve_api_key() {
        let _guard = OAuthGuard::new();
        unsafe {
            std::env::remove_var("CLAUDE_CODE_USE_BEDROCK");
            std::env::remove_var("CLAUDE_CODE_USE_VERTEX");
            std::env::remove_var("CLAUDE_CODE_USE_COPILOT");
            std::env::remove_var("COPILOT_PROXY_URL");
            std::env::set_var("ANTHROPIC_API_KEY", "sk-test-resolve");
        }
        let provider = resolve_api_provider().unwrap();
        match provider {
            ApiProvider::Anthropic { api_key, base_url } => {
                assert_eq!(api_key, "sk-test-resolve");
                assert_eq!(base_url, "https://api.anthropic.com");
            }
            // System keychain may have real OAuth tokens that take priority
            ApiProvider::OAuth { .. } => {}
            other => panic!("Expected Anthropic or OAuth, got {other:?}"),
        }
        unsafe {
            std::env::remove_var("ANTHROPIC_API_KEY");
        }
    }

    #[test]
    #[serial]
    fn test_resolve_nothing() {
        let _guard = OAuthGuard::new();
        unsafe {
            std::env::remove_var("CLAUDE_CODE_USE_BEDROCK");
            std::env::remove_var("CLAUDE_CODE_USE_VERTEX");
            std::env::remove_var("CLAUDE_CODE_USE_COPILOT");
            std::env::remove_var("COPILOT_PROXY_URL");
            std::env::remove_var("ANTHROPIC_API_KEY");
        }
        let result = resolve_api_provider();
        // With no env vars and no OAuth file, should fail (unless keychain has something)
        if let Ok(provider) = result {
            match provider {
                ApiProvider::OAuth { .. } | ApiProvider::Anthropic { .. } => {
                    // Acceptable: real credentials on dev machine via keychain
                }
                other => panic!("Unexpected provider without env vars: {other:?}"),
            }
        }
    }

    #[test]
    fn test_get_api_base_url() {
        let provider = ApiProvider::Anthropic {
            api_key: "k".into(),
            base_url: "https://custom.api.com".into(),
        };
        assert_eq!(get_api_base_url(&provider), "https://custom.api.com");

        let bedrock = ApiProvider::Bedrock {
            region: "us-west-2".into(),
            profile: None,
        };
        assert!(get_api_base_url(&bedrock).contains("us-west-2"));
    }

    #[test]
    fn test_get_api_base_url_vertex() {
        let vertex = ApiProvider::Vertex {
            project_id: "proj-123".into(),
            region: "europe-west4".into(),
        };
        let url = get_api_base_url(&vertex);
        assert!(url.contains("europe-west4"), "should contain region");
        assert!(url.contains("proj-123"), "should contain project id");
        assert!(url.contains("aiplatform.googleapis.com"), "should be vertex endpoint");
    }

    #[test]
    fn test_get_api_base_url_oauth() {
        let oauth = ApiProvider::OAuth {
            access_token: "tok".into(),
            refresh_token: "ref".into(),
        };
        assert_eq!(get_api_base_url(&oauth), "https://api.anthropic.com");
    }

    #[test]
    #[serial]
    fn test_resolve_bedrock_default_region() {
        unsafe {
            std::env::set_var("CLAUDE_CODE_USE_BEDROCK", "1");
            std::env::remove_var("AWS_REGION");
            std::env::remove_var("AWS_DEFAULT_REGION");
        }
        let provider = resolve_api_provider().unwrap();
        match provider {
            ApiProvider::Bedrock { region, .. } => assert_eq!(region, "us-east-1"),
            other => panic!("Expected Bedrock, got {other:?}"),
        }
        unsafe {
            std::env::remove_var("CLAUDE_CODE_USE_BEDROCK");
        }
    }

    #[test]
    #[serial]
    fn test_resolve_bedrock_with_profile() {
        unsafe {
            std::env::set_var("CLAUDE_CODE_USE_BEDROCK", "1");
            std::env::set_var("AWS_REGION", "ap-southeast-1");
            std::env::set_var("AWS_PROFILE", "dev-profile");
        }
        let provider = resolve_api_provider().unwrap();
        match provider {
            ApiProvider::Bedrock { region, profile } => {
                assert_eq!(region, "ap-southeast-1");
                assert_eq!(profile, Some("dev-profile".into()));
            }
            other => panic!("Expected Bedrock, got {other:?}"),
        }
        unsafe {
            std::env::remove_var("CLAUDE_CODE_USE_BEDROCK");
            std::env::remove_var("AWS_REGION");
            std::env::remove_var("AWS_PROFILE");
        }
    }

    #[test]
    #[serial]
    fn test_resolve_api_key_custom_base_url() {
        let _guard = OAuthGuard::new();
        unsafe {
            std::env::remove_var("CLAUDE_CODE_USE_BEDROCK");
            std::env::remove_var("CLAUDE_CODE_USE_VERTEX");
            std::env::remove_var("CLAUDE_CODE_USE_COPILOT");
            std::env::remove_var("COPILOT_PROXY_URL");
            std::env::set_var("ANTHROPIC_API_KEY", "sk-custom");
            std::env::set_var("ANTHROPIC_BASE_URL", "https://proxy.example.com");
        }
        let provider = resolve_api_provider().unwrap();
        match provider {
            ApiProvider::Anthropic { api_key, base_url } => {
                assert_eq!(api_key, "sk-custom");
                assert_eq!(base_url, "https://proxy.example.com");
            }
            other => panic!("Expected Anthropic, got {other:?}"),
        }
        unsafe {
            std::env::remove_var("ANTHROPIC_API_KEY");
            std::env::remove_var("ANTHROPIC_BASE_URL");
        }
    }

    #[test]
    fn test_api_provider_serde_roundtrip() {
        let provider = ApiProvider::Anthropic {
            api_key: "sk-test".into(),
            base_url: "https://api.anthropic.com".into(),
        };
        let json = serde_json::to_string(&provider).expect("serialize");
        let deser: ApiProvider = serde_json::from_str(&json).expect("deserialize");
        match deser {
            ApiProvider::Anthropic { api_key, base_url } => {
                assert_eq!(api_key, "sk-test");
                assert_eq!(base_url, "https://api.anthropic.com");
            }
            other => panic!("Expected Anthropic, got {other:?}"),
        }
    }

    #[test]
    #[serial]
    fn test_resolve_copilot_proxy_default() {
        let _guard = OAuthGuard::new();
        unsafe {
            std::env::remove_var("CLAUDE_CODE_USE_BEDROCK");
            std::env::remove_var("CLAUDE_CODE_USE_VERTEX");
            std::env::remove_var("ANTHROPIC_API_KEY");
            std::env::remove_var("COPILOT_PROXY_URL");
            std::env::remove_var("COPILOT_PROXY_KEY");
            std::env::set_var("CLAUDE_CODE_USE_COPILOT", "1");
        }
        let provider = resolve_api_provider().expect("should resolve CopilotProxy");
        match provider {
            ApiProvider::CopilotProxy { proxy_url, api_key } => {
                assert_eq!(proxy_url, "http://127.0.0.1:23333/api/anthropic");
                assert!(api_key.is_none());
            }
            other => panic!("Expected CopilotProxy, got {other:?}"),
        }
        unsafe {
            std::env::remove_var("CLAUDE_CODE_USE_COPILOT");
        }
    }

    #[test]
    #[serial]
    fn test_resolve_copilot_proxy_custom_url() {
        let _guard = OAuthGuard::new();
        unsafe {
            std::env::remove_var("CLAUDE_CODE_USE_BEDROCK");
            std::env::remove_var("CLAUDE_CODE_USE_VERTEX");
            std::env::remove_var("ANTHROPIC_API_KEY");
            std::env::remove_var("CLAUDE_CODE_USE_COPILOT");
            std::env::set_var("COPILOT_PROXY_URL", "http://localhost:9999/api/anthropic");
        }
        let provider = resolve_api_provider().expect("should resolve CopilotProxy");
        match provider {
            ApiProvider::CopilotProxy { proxy_url, .. } => {
                assert_eq!(proxy_url, "http://localhost:9999/api/anthropic");
            }
            other => panic!("Expected CopilotProxy, got {other:?}"),
        }
        unsafe {
            std::env::remove_var("COPILOT_PROXY_URL");
        }
    }

    #[test]
    #[serial]
    fn test_resolve_copilot_proxy_with_key() {
        let _guard = OAuthGuard::new();
        unsafe {
            std::env::remove_var("CLAUDE_CODE_USE_BEDROCK");
            std::env::remove_var("CLAUDE_CODE_USE_VERTEX");
            std::env::remove_var("ANTHROPIC_API_KEY");
            std::env::set_var("CLAUDE_CODE_USE_COPILOT", "1");
            std::env::set_var("COPILOT_PROXY_KEY", "my-proxy-key");
        }
        let provider = resolve_api_provider().expect("should resolve CopilotProxy");
        match provider {
            ApiProvider::CopilotProxy { api_key, .. } => {
                assert_eq!(api_key, Some("my-proxy-key".to_string()));
            }
            other => panic!("Expected CopilotProxy, got {other:?}"),
        }
        unsafe {
            std::env::remove_var("CLAUDE_CODE_USE_COPILOT");
            std::env::remove_var("COPILOT_PROXY_KEY");
        }
    }

    #[test]
    fn test_copilot_proxy_base_url() {
        let provider = ApiProvider::CopilotProxy {
            proxy_url: "http://127.0.0.1:23333/api/anthropic".into(),
            api_key: None,
        };
        assert_eq!(
            get_api_base_url(&provider),
            "http://127.0.0.1:23333/api/anthropic"
        );
    }

    #[test]
    fn test_copilot_proxy_serde_roundtrip() {
        let provider = ApiProvider::CopilotProxy {
            proxy_url: "http://127.0.0.1:23333/api/anthropic".into(),
            api_key: Some("key-123".into()),
        };
        let json = serde_json::to_string(&provider).expect("serialize");
        let deser: ApiProvider = serde_json::from_str(&json).expect("deserialize");
        match deser {
            ApiProvider::CopilotProxy { proxy_url, api_key } => {
                assert_eq!(proxy_url, "http://127.0.0.1:23333/api/anthropic");
                assert_eq!(api_key, Some("key-123".to_string()));
            }
            other => panic!("Expected CopilotProxy, got {other:?}"),
        }
    }
}
