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
}

/// Resolve the API provider from environment variables and stored credentials.
///
/// Priority order:
/// 1. `CLAUDE_CODE_USE_BEDROCK=1` -> Bedrock
/// 2. `CLAUDE_CODE_USE_VERTEX=1` -> Vertex
/// 3. Valid (non-expired) OAuth tokens -> OAuth
/// 4. API key (env / config file / keychain) -> Anthropic
/// 5. Error
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

    // 3. OAuth tokens (file or keychain)
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

    // 4. API key
    if let Some(api_key) = crate::api_key::get_api_key() {
        let base_url = std::env::var("ANTHROPIC_BASE_URL")
            .unwrap_or_else(|_| "https://api.anthropic.com".into());
        debug!("Resolved provider: Anthropic (API key)");
        return Ok(ApiProvider::Anthropic { api_key, base_url });
    }

    // 5. Nothing found
    anyhow::bail!(
        "No API credentials found. Set ANTHROPIC_API_KEY, \
         run `claude auth login` for OAuth, or configure Bedrock/Vertex."
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

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
        unsafe {
            std::env::remove_var("CLAUDE_CODE_USE_BEDROCK");
            std::env::remove_var("CLAUDE_CODE_USE_VERTEX");
            std::env::set_var("ANTHROPIC_API_KEY", "sk-test-resolve");
        }
        let provider = resolve_api_provider().unwrap();
        match provider {
            ApiProvider::Anthropic { api_key, base_url } => {
                assert_eq!(api_key, "sk-test-resolve");
                assert_eq!(base_url, "https://api.anthropic.com");
            }
            other => panic!("Expected Anthropic, got {other:?}"),
        }
        unsafe {
            std::env::remove_var("ANTHROPIC_API_KEY");
        }
    }

    #[test]
    #[serial]
    fn test_resolve_nothing() {
        unsafe {
            std::env::remove_var("CLAUDE_CODE_USE_BEDROCK");
            std::env::remove_var("CLAUDE_CODE_USE_VERTEX");
            std::env::remove_var("ANTHROPIC_API_KEY");
        }
        let result = resolve_api_provider();
        assert!(result.is_err());
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
}
