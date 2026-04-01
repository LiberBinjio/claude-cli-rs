//! Core query loop: API call → stream → tool execution → repeat.

use crate::engine::QueryEvent;
use claude_api::streaming::{ContentDelta, StreamEvent};
use claude_api::ApiClient;
use claude_core::message::{ContentBlock, Message, Role, ToolResultContent};
use crate::tool_set::ToolSet;
use futures_util::StreamExt;
use tokio::sync::mpsc;
use tracing::{debug, warn};

/// Maximum number of tool-use rounds before stopping.
const MAX_TOOL_ROUNDS: usize = 25;

/// Accumulator for streamed content blocks.
#[derive(Debug, Default)]
struct BlockAccumulator {
    content_type: String,
    text: String,
    tool_id: String,
    tool_name: String,
    input_json: String,
}

/// Run the full query loop until completion or max rounds.
///
/// This function:
/// 1. Calls the API with the current messages
/// 2. Streams the response, emitting `QueryEvent`s
/// 3. If the response contains tool_use blocks, executes tools
/// 4. Appends tool results and loops back to step 1
pub async fn run_query_loop(
    api_client: &ApiClient,
    messages: &mut Vec<Message>,
    tool_set: &ToolSet,
    system_prompt: &str,
    _model: &str,
    cwd: &std::path::Path,
    event_tx: mpsc::Sender<QueryEvent>,
) -> anyhow::Result<()> {
    for round in 0..MAX_TOOL_ROUNDS {
        debug!(round, "Starting query loop round");

        // Build tool schemas
        let tools = tool_set.to_api_schemas();

        // Call API
        let stream = api_client
            .send_message(messages, system_prompt, &tools, 8192)
            .await
            .map_err(|e| anyhow::anyhow!("API error: {e}"))?;

        // Consume stream
        let (assistant_blocks, stop_reason) =
            consume_stream(stream, &event_tx).await?;

        // Build assistant message
        let mut content = Vec::new();
        let mut has_tool_use = false;

        for block in &assistant_blocks {
            match block.content_type.as_str() {
                "text" if !block.text.is_empty() => {
                    content.push(ContentBlock::Text {
                        text: block.text.clone(),
                    });
                }
                "tool_use" if !block.tool_id.is_empty() => {
                    has_tool_use = true;
                    let input: serde_json::Value =
                        serde_json::from_str(&block.input_json)
                            .unwrap_or(serde_json::Value::Object(Default::default()));
                    content.push(ContentBlock::ToolUse {
                        id: block.tool_id.clone(),
                        name: block.tool_name.clone(),
                        input,
                    });
                }
                _ => {}
            }
        }

        if !content.is_empty() {
            messages.push(Message {
                role: Role::Assistant,
                content,
                cache_control: None,
            });
        }

        // If no tool_use, we're done
        if !has_tool_use {
            debug!(stop_reason = ?stop_reason, "Query complete — no tool use");
            return Ok(());
        }

        // Execute tools and build result message
        let mut tool_results = Vec::new();
        for block in &assistant_blocks {
            if block.content_type != "tool_use" || block.tool_id.is_empty() {
                continue;
            }

            let _ = event_tx
                .send(QueryEvent::ToolStart {
                    tool_name: block.tool_name.clone(),
                    tool_use_id: block.tool_id.clone(),
                })
                .await;

            let input: serde_json::Value =
                serde_json::from_str(&block.input_json)
                    .unwrap_or(serde_json::Value::Object(Default::default()));

            let (output, is_error) = execute_tool(tool_set, &block.tool_name, &input, cwd).await;

            let _ = event_tx
                .send(QueryEvent::ToolEnd {
                    tool_use_id: block.tool_id.clone(),
                    result: output.clone(),
                    is_error,
                })
                .await;

            tool_results.push(ContentBlock::ToolResult {
                tool_use_id: block.tool_id.clone(),
                content: vec![ToolResultContent {
                    content_type: "text".into(),
                    text: Some(output),
                }],
                is_error: Some(is_error),
            });
        }

        // Add tool results as a user message
        messages.push(Message {
            role: Role::User,
            content: tool_results,
            cache_control: None,
        });

        // Continue loop for next API call
    }

    warn!("Max tool rounds ({MAX_TOOL_ROUNDS}) exceeded");
    Err(anyhow::anyhow!(
        "Maximum tool execution rounds ({MAX_TOOL_ROUNDS}) exceeded"
    ))
}

/// Consume a stream of `StreamEvent`s, collecting block accumulators.
async fn consume_stream(
    stream: impl futures_util::Stream<Item = Result<StreamEvent, claude_api::ApiError>>,
    event_tx: &mpsc::Sender<QueryEvent>,
) -> anyhow::Result<(Vec<BlockAccumulator>, Option<String>)> {
    let mut blocks: Vec<BlockAccumulator> = Vec::new();
    let mut stop_reason: Option<String> = None;

    tokio::pin!(stream);

    while let Some(event) = stream.next().await {
        let event = event.map_err(|e| anyhow::anyhow!("Stream error: {e}"))?;

        match event {
            StreamEvent::ContentBlockStart {
                index,
                content_type,
                tool_id,
                tool_name,
            } => {
                // Ensure the blocks vec is large enough
                while blocks.len() <= index {
                    blocks.push(BlockAccumulator::default());
                }
                blocks[index].content_type = content_type;
                if let Some(id) = tool_id {
                    blocks[index].tool_id = id;
                }
                if let Some(name) = tool_name {
                    blocks[index].tool_name = name;
                }
            }
            StreamEvent::ContentBlockDelta { index, delta } => {
                if index < blocks.len() {
                    match delta {
                        ContentDelta::TextDelta(text) => {
                            blocks[index].text.push_str(&text);
                            let _ = event_tx
                                .send(QueryEvent::StreamDelta { text })
                                .await;
                        }
                        ContentDelta::InputJsonDelta(json) => {
                            blocks[index].input_json.push_str(&json);
                        }
                        ContentDelta::ThinkingDelta(_) | ContentDelta::SignatureDelta(_) => {}
                    }
                }
            }
            StreamEvent::MessageDelta {
                stop_reason: reason,
                ..
            } => {
                stop_reason = reason;
            }
            StreamEvent::Error(msg) => {
                return Err(anyhow::anyhow!("SSE error: {msg}"));
            }
            _ => {}
        }
    }

    Ok((blocks, stop_reason))
}

/// Execute a single tool by name.
async fn execute_tool(
    tool_set: &ToolSet,
    name: &str,
    input: &serde_json::Value,
    cwd: &std::path::Path,
) -> (String, bool) {
    let tool = match tool_set.find(name) {
        Some(t) => t,
        None => return (format!("Tool not found: {name}"), true),
    };

    let mut ctx = claude_core::tool::ToolUseContext {
        cwd: cwd.to_path_buf(),
        permission_mode: claude_core::permission::PermissionMode::Default,
        tool_use_id: uuid::Uuid::new_v4().to_string(),
        session_id: "session".to_string(),
    };

    match tool.call(input.clone(), &mut ctx).await {
        Ok(result) => {
            let text = result
                .content
                .iter()
                .filter_map(|c| c.text.as_deref())
                .collect::<Vec<_>>()
                .join("\n");
            (text, result.is_error)
        }
        Err(e) => (format!("Tool error: {e}"), true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_accumulator_default() {
        let acc = BlockAccumulator::default();
        assert!(acc.content_type.is_empty());
        assert!(acc.text.is_empty());
        assert!(acc.tool_id.is_empty());
    }

    #[tokio::test]
    async fn test_consume_stream_text_only() {
        let events = vec![
            Ok(StreamEvent::ContentBlockStart {
                index: 0,
                content_type: "text".into(),
                tool_id: None,
                tool_name: None,
            }),
            Ok(StreamEvent::ContentBlockDelta {
                index: 0,
                delta: ContentDelta::TextDelta("Hello ".into()),
            }),
            Ok(StreamEvent::ContentBlockDelta {
                index: 0,
                delta: ContentDelta::TextDelta("world".into()),
            }),
            Ok(StreamEvent::ContentBlockStop { index: 0 }),
            Ok(StreamEvent::MessageDelta {
                stop_reason: Some("end_turn".into()),
                usage: Default::default(),
            }),
            Ok(StreamEvent::MessageStop),
        ];

        let stream = futures_util::stream::iter(events);
        let (tx, mut rx) = mpsc::channel(32);
        let (blocks, stop) = consume_stream(stream, &tx).await.unwrap();

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].text, "Hello world");
        assert_eq!(stop.as_deref(), Some("end_turn"));

        // Check that deltas were emitted
        let ev1 = rx.recv().await.unwrap();
        assert!(matches!(ev1, QueryEvent::StreamDelta { text } if text == "Hello "));
        let ev2 = rx.recv().await.unwrap();
        assert!(matches!(ev2, QueryEvent::StreamDelta { text } if text == "world"));
    }

    #[tokio::test]
    async fn test_consume_stream_with_tool_use() {
        let events = vec![
            Ok(StreamEvent::ContentBlockStart {
                index: 0,
                content_type: "text".into(),
                tool_id: None,
                tool_name: None,
            }),
            Ok(StreamEvent::ContentBlockDelta {
                index: 0,
                delta: ContentDelta::TextDelta("Let me check.".into()),
            }),
            Ok(StreamEvent::ContentBlockStop { index: 0 }),
            Ok(StreamEvent::ContentBlockStart {
                index: 1,
                content_type: "tool_use".into(),
                tool_id: Some("toolu_1".into()),
                tool_name: Some("Bash".into()),
            }),
            Ok(StreamEvent::ContentBlockDelta {
                index: 1,
                delta: ContentDelta::InputJsonDelta(r#"{"command""#.into()),
            }),
            Ok(StreamEvent::ContentBlockDelta {
                index: 1,
                delta: ContentDelta::InputJsonDelta(r#": "ls"}"#.into()),
            }),
            Ok(StreamEvent::ContentBlockStop { index: 1 }),
            Ok(StreamEvent::MessageDelta {
                stop_reason: Some("tool_use".into()),
                usage: Default::default(),
            }),
            Ok(StreamEvent::MessageStop),
        ];

        let stream = futures_util::stream::iter(events);
        let (tx, _rx) = mpsc::channel(32);
        let (blocks, stop) = consume_stream(stream, &tx).await.unwrap();

        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].text, "Let me check.");
        assert_eq!(blocks[1].content_type, "tool_use");
        assert_eq!(blocks[1].tool_id, "toolu_1");
        assert_eq!(blocks[1].tool_name, "Bash");

        let input: serde_json::Value = serde_json::from_str(&blocks[1].input_json).unwrap();
        assert_eq!(input["command"], "ls");
        assert_eq!(stop.as_deref(), Some("tool_use"));
    }

    #[tokio::test]
    async fn test_consume_stream_error() {
        let events = vec![Ok(StreamEvent::Error("overloaded".into()))];
        let stream = futures_util::stream::iter(events);
        let (tx, _rx) = mpsc::channel(32);
        let result = consume_stream(stream, &tx).await;
        assert!(result.is_err());
    }
}
