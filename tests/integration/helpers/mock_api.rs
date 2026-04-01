//! Minimal mock Anthropic API server for integration testing.
#![allow(dead_code)]

/// Stub mock server that captures requests for assertions.
pub struct MockAnthropicApi {
    /// The base URL the mock listens on (e.g. `http://127.0.0.1:<port>`).
    pub base_url: String,
}

impl MockAnthropicApi {
    /// Create a placeholder mock (does not start a real server).
    pub fn new() -> Self {
        Self {
            base_url: "http://127.0.0.1:0".to_owned(),
        }
    }
}
