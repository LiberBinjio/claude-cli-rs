//! Anthropic Messages API client.

use crate::errors::ApiError;
use crate::streaming::{parse_sse_stream, StreamEvent};
use claude_auth::ApiProvider;
use claude_core::config::AppConfig;
use claude_core::message::Message;
use futures_util::Stream;
use tracing::debug;

/// Client for the Anthropic Messages API.
pub struct ApiClient {
    http: reqwest::Client,
    provider: ApiProvider,
    config: AppConfig,
}

impl ApiClient {
    /// Create a new API client.
    #[must_use]
    pub fn new(provider: ApiProvider, config: AppConfig) -> Self {
        Self {
            http: reqwest::Client::new(),
            provider,
            config,
        }
    }

    /// Send a streaming message request.
    ///
    /// Returns a stream of `StreamEvent`s as the response arrives.
    pub async fn send_message(
        &self,
        messages: &[Message],
        system: &str,
        tools: &[serde_json::Value],
        max_tokens: u32,
    ) -> Result<impl Stream<Item = Result<StreamEvent, ApiError>>, ApiError> {
        let (url, headers) = self.build_request_info();
        let model = &self.config.model;

        let mut body = serde_json::json!({
            "model": model,
            "max_tokens": max_tokens,
            "stream": true,
            "messages": messages,
        });

        if !system.is_empty() {
            body["system"] = serde_json::Value::String(system.to_string());
        }

        if !tools.is_empty() {
            body["tools"] = serde_json::Value::Array(tools.to_vec());
        }

        debug!(url = %url, model = %model, msg_count = messages.len(), "Sending API request");

        let mut request = self.http.post(&url).json(&body);

        for (key, value) in &headers {
            request = request.header(key.as_str(), value.as_str());
        }

        let response = request.send().await?;
        let status = response.status().as_u16();

        if status != 200 {
            let body_text = response.text().await.unwrap_or_default();
            return Err(ApiError::from_status(status, &body_text));
        }

        Ok(parse_sse_stream(response))
    }

    /// Build the URL and headers for the request based on the provider.
    fn build_request_info(&self) -> (String, Vec<(String, String)>) {
        match &self.provider {
            ApiProvider::Anthropic { api_key, base_url } => {
                let url = if let Some(custom) = &self.config.custom_api_url {
                    format!("{custom}/v1/messages")
                } else {
                    format!("{base_url}/v1/messages")
                };
                let headers = vec![
                    ("x-api-key".to_string(), api_key.clone()),
                    (
                        "anthropic-version".to_string(),
                        "2023-06-01".to_string(),
                    ),
                    (
                        "content-type".to_string(),
                        "application/json".to_string(),
                    ),
                ];
                (url, headers)
            }
            ApiProvider::OAuth {
                access_token,
                refresh_token: _,
            } => {
                let url = self
                    .config
                    .custom_api_url
                    .as_deref()
                    .unwrap_or("https://api.anthropic.com")
                    .to_string()
                    + "/v1/messages";
                let headers = vec![
                    (
                        "authorization".to_string(),
                        format!("Bearer {access_token}"),
                    ),
                    (
                        "anthropic-version".to_string(),
                        "2023-06-01".to_string(),
                    ),
                    (
                        "content-type".to_string(),
                        "application/json".to_string(),
                    ),
                ];
                (url, headers)
            }
            ApiProvider::Bedrock { region, .. } => {
                let url = format!(
                    "https://bedrock-runtime.{region}.amazonaws.com/model/{}/invoke-with-response-stream",
                    self.config.model
                );
                let headers = vec![(
                    "content-type".to_string(),
                    "application/json".to_string(),
                )];
                (url, headers)
            }
            ApiProvider::Vertex {
                project_id, region, ..
            } => {
                let url = format!(
                    "https://{region}-aiplatform.googleapis.com/v1/projects/{project_id}/locations/{region}/publishers/anthropic/models/{}:streamRawPredict",
                    self.config.model
                );
                let headers = vec![(
                    "content-type".to_string(),
                    "application/json".to_string(),
                )];
                (url, headers)
            }
            ApiProvider::CopilotProxy { proxy_url, api_key } => {
                let url = format!("{proxy_url}/v1/messages");
                let mut headers = vec![
                    ("anthropic-version".to_string(), "2023-06-01".to_string()),
                    ("content-type".to_string(), "application/json".to_string()),
                ];
                if let Some(key) = api_key {
                    headers.push(("x-api-key".to_string(), key.clone()));
                }
                (url, headers)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_client(provider: ApiProvider) -> ApiClient {
        ApiClient::new(provider, AppConfig::default())
    }

    #[test]
    fn test_anthropic_url() {
        let client = make_client(ApiProvider::Anthropic {
            api_key: "sk-test".into(),
            base_url: "https://api.anthropic.com".into(),
        });
        let (url, headers) = client.build_request_info();
        assert!(url.contains("/v1/messages"));
        assert!(headers.iter().any(|(k, v)| k == "x-api-key" && v == "sk-test"));
        assert!(headers
            .iter()
            .any(|(k, v)| k == "anthropic-version" && v == "2023-06-01"));
    }

    #[test]
    fn test_oauth_url() {
        let client = make_client(ApiProvider::OAuth {
            access_token: "tok_123".into(),
            refresh_token: "ref_123".into(),
        });
        let (url, headers) = client.build_request_info();
        assert!(url.contains("/v1/messages"));
        assert!(headers
            .iter()
            .any(|(k, v)| k == "authorization" && v == "Bearer tok_123"));
    }

    #[test]
    fn test_bedrock_url() {
        let client = make_client(ApiProvider::Bedrock {
            region: "us-east-1".into(),
            profile: None,
        });
        let (url, _) = client.build_request_info();
        assert!(url.contains("bedrock-runtime"));
        assert!(url.contains("us-east-1"));
    }

    #[test]
    fn test_vertex_url() {
        let client = make_client(ApiProvider::Vertex {
            project_id: "my-project".into(),
            region: "us-central1".into(),
        });
        let (url, _) = client.build_request_info();
        assert!(url.contains("aiplatform.googleapis.com"));
        assert!(url.contains("my-project"));
    }

    #[test]
    fn test_custom_api_url() {
        let mut config = AppConfig::default();
        config.custom_api_url = Some("https://custom.api.com".into());
        let client = ApiClient::new(
            ApiProvider::Anthropic {
                api_key: "sk-test".into(),
                base_url: "https://api.anthropic.com".into(),
            },
            config,
        );
        let (url, _) = client.build_request_info();
        assert!(url.starts_with("https://custom.api.com/v1/messages"));
    }

    #[test]
    fn test_copilot_proxy_url() {
        let client = make_client(ApiProvider::CopilotProxy {
            proxy_url: "http://127.0.0.1:23333/api/anthropic".into(),
            api_key: Some("test-key".into()),
        });
        let (url, headers) = client.build_request_info();
        assert_eq!(url, "http://127.0.0.1:23333/api/anthropic/v1/messages");
        assert!(headers.iter().any(|(k, v)| k == "x-api-key" && v == "test-key"));
        assert!(headers.iter().any(|(k, v)| k == "anthropic-version" && v == "2023-06-01"));
    }

    #[test]
    fn test_copilot_proxy_url_no_key() {
        let client = make_client(ApiProvider::CopilotProxy {
            proxy_url: "http://localhost:9999/api/anthropic".into(),
            api_key: None,
        });
        let (url, headers) = client.build_request_info();
        assert_eq!(url, "http://localhost:9999/api/anthropic/v1/messages");
        assert!(!headers.iter().any(|(k, _)| k == "x-api-key"));
    }
}
