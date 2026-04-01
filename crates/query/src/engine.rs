//! QueryEngine: orchestrates conversations between user, API, and tools.

use crate::compact::{compact_messages, should_compact, CompactConfig};
use crate::query_loop::run_query_loop;
use crate::system_prompt::build_system_prompt;
use crate::tool_set::ToolSet;
use claude_api::ApiClient;
use claude_commands::{CommandContext, CommandRegistry, CommandResult};
use claude_core::config::AppConfig;
use claude_core::message::Message;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, warn};

/// Events emitted by the query engine to update the UI.
#[derive(Debug, Clone)]
pub enum QueryEvent {
    /// Incremental text from the assistant.
    StreamDelta {
        /// The text fragment.
        text: String,
    },
    /// A tool has started executing.
    ToolStart {
        /// Name of the tool.
        tool_name: String,
        /// Unique id for this invocation.
        tool_use_id: String,
    },
    /// A tool has finished executing.
    ToolEnd {
        /// Unique id matching the [`QueryEvent::ToolStart`].
        tool_use_id: String,
        /// Output of the tool.
        result: String,
        /// Whether the tool reported an error.
        is_error: bool,
    },
    /// The query has completed successfully.
    QueryComplete,
    /// An error occurred.
    Error {
        /// Human-readable error description.
        message: String,
    },
}

/// Central engine managing a conversation session.
pub struct QueryEngine {
    messages: Vec<Message>,
    api_client: Arc<ApiClient>,
    tool_set: Arc<ToolSet>,
    command_registry: Arc<CommandRegistry>,
    system_prompt: String,
    model: String,
    cwd: std::path::PathBuf,
    compact_config: CompactConfig,
}

impl QueryEngine {
    /// Create a new `QueryEngine`.
    #[must_use]
    pub fn new(
        api_client: Arc<ApiClient>,
        tool_set: Arc<ToolSet>,
        command_registry: Arc<CommandRegistry>,
        cwd: std::path::PathBuf,
    ) -> Self {
        let tool_names = tool_set.names();
        let config = AppConfig::default();
        let system_prompt = build_system_prompt(&config, &cwd, &tool_names);
        let model = config.model.clone();

        Self {
            messages: Vec::new(),
            api_client,
            tool_set,
            command_registry,
            system_prompt,
            model,
            cwd,
            compact_config: CompactConfig::default(),
        }
    }

    /// Set the model to use.
    pub fn set_model(&mut self, model: &str) {
        self.model = model.to_string();
    }

    /// Override the system prompt.
    pub fn set_system_prompt(&mut self, prompt: String) {
        self.system_prompt = prompt;
    }

    /// Get the current message history.
    #[must_use]
    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    /// Clear the message history.
    pub fn clear_messages(&mut self) {
        self.messages.clear();
    }

    /// Process user input: dispatch slash commands or run a query.
    pub async fn process_user_input(
        &mut self,
        input: String,
        event_tx: mpsc::Sender<QueryEvent>,
    ) -> anyhow::Result<()> {
        let trimmed = input.trim();

        // Check for slash commands
        if trimmed.starts_with('/') {
            if let Some((cmd, args)) = self.command_registry.find(trimmed) {
                debug!(command = cmd.name(), "Executing slash command");
                let mut ctx = CommandContext {
                    placeholder_state: (),
                    event_tx: None,
                };
                match cmd.execute(&args, &mut ctx).await {
                    Ok(result) => {
                        let msg = match result {
                            CommandResult::Handled(text) => {
                                text.unwrap_or_else(|| "Command executed.".to_string())
                            }
                            CommandResult::SendToApi(text) => {
                                return self.run_query(text, event_tx).await;
                            }
                        };
                        let _ = event_tx
                            .send(QueryEvent::StreamDelta { text: msg })
                            .await;
                        let _ = event_tx.send(QueryEvent::QueryComplete).await;
                        return Ok(());
                    }
                    Err(e) => {
                        let _ = event_tx
                            .send(QueryEvent::Error {
                                message: format!("Command error: {e}"),
                            })
                            .await;
                        return Ok(());
                    }
                }
            }
        }

        // Regular query
        self.run_query(input, event_tx).await
    }

    /// Run a query against the API.
    async fn run_query(
        &mut self,
        input: String,
        event_tx: mpsc::Sender<QueryEvent>,
    ) -> anyhow::Result<()> {
        // Add user message
        self.messages.push(Message::user(&input));

        // Check if compaction needed
        if should_compact(&self.messages, &self.compact_config) {
            debug!("Compacting conversation context");
            self.messages = compact_messages(&self.messages, &self.compact_config);
        }

        // Run the query loop
        match run_query_loop(
            &self.api_client,
            &mut self.messages,
            &self.tool_set,
            &self.system_prompt,
            &self.model,
            &self.cwd,
            event_tx.clone(),
        )
        .await
        {
            Ok(()) => {
                let _ = event_tx.send(QueryEvent::QueryComplete).await;
            }
            Err(e) => {
                warn!(error = %e, "Query loop error");
                let _ = event_tx
                    .send(QueryEvent::Error {
                        message: e.to_string(),
                    })
                    .await;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claude_auth::ApiProvider;

    fn make_engine() -> QueryEngine {
        let config = AppConfig::default();
        let provider = ApiProvider::Anthropic {
            api_key: "test-key".into(),
            base_url: "https://api.anthropic.com".into(),
        };
        let api_client = Arc::new(ApiClient::new(provider, config));
        let tool_set = Arc::new(ToolSet::new());
        let cmd_registry = Arc::new(CommandRegistry::new());

        QueryEngine::new(
            api_client,
            tool_set,
            cmd_registry,
            std::path::PathBuf::from("."),
        )
    }

    #[test]
    fn test_new_engine() {
        let engine = make_engine();
        assert!(engine.messages().is_empty());
        assert!(!engine.system_prompt.is_empty());
    }

    #[test]
    fn test_set_model() {
        let mut engine = make_engine();
        engine.set_model("claude-haiku-4-20250414");
        assert_eq!(engine.model, "claude-haiku-4-20250414");
    }

    #[test]
    fn test_clear_messages() {
        let mut engine = make_engine();
        engine.messages.push(Message::user("hello"));
        assert_eq!(engine.messages().len(), 1);
        engine.clear_messages();
        assert!(engine.messages().is_empty());
    }

    #[test]
    fn test_set_system_prompt() {
        let mut engine = make_engine();
        engine.set_system_prompt("Custom prompt".to_string());
        assert_eq!(engine.system_prompt, "Custom prompt");
    }

    #[test]
    fn test_new_engine_system_prompt_contains_key_sections() {
        let engine = make_engine();
        assert!(engine.system_prompt.contains("Claude"));
        assert!(engine.system_prompt.contains("Platform:"));
        assert!(engine.system_prompt.contains("Architecture:"));
    }

    #[test]
    fn test_new_engine_model_is_default() {
        let engine = make_engine();
        assert_eq!(engine.model, "claude-sonnet-4-20250514");
    }

    #[test]
    fn test_new_engine_cwd_matches() {
        let engine = make_engine();
        assert_eq!(engine.cwd, std::path::PathBuf::from("."));
    }

    #[test]
    fn test_new_engine_compact_config_defaults() {
        let engine = make_engine();
        assert_eq!(engine.compact_config.threshold, 100_000);
        assert_eq!(engine.compact_config.keep_recent, 10);
    }

    #[tokio::test]
    async fn test_process_unknown_slash_command() {
        let mut engine = make_engine();
        let (tx, mut rx) = mpsc::channel(32);
        engine
            .process_user_input("/nonexistent".to_string(), tx)
            .await
            .unwrap();
        // Unknown slash command is treated as a regular query, which fails
        // because the API key is fake. We should get an error event.
        let event = rx.recv().await.unwrap();
        assert!(matches!(
            event,
            QueryEvent::Error { .. } | QueryEvent::QueryComplete
        ));
    }

    #[tokio::test]
    async fn test_process_regular_input_adds_user_message() {
        let mut engine = make_engine();
        let (tx, mut rx) = mpsc::channel(32);
        // This will fail on the API call but should at least add the user message
        let _ = engine.process_user_input("hello".to_string(), tx).await;
        assert!(!engine.messages().is_empty());
        assert_eq!(engine.messages()[0].role, claude_core::message::Role::User);
        assert_eq!(engine.messages()[0].text(), "hello");
        // Drain events
        while rx.try_recv().is_ok() {}
    }
}
