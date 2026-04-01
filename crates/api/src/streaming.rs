//! SSE (Server-Sent Events) parsing and stream event types.

use crate::errors::ApiError;
use futures_util::{Stream, StreamExt};
use serde::{Deserialize, Serialize};

/// A parsed SSE event from the Anthropic Messages API.
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// Start of a new message.
    MessageStart {
        /// Message ID.
        id: String,
        /// Model used.
        model: String,
    },
    /// Start of a content block.
    ContentBlockStart {
        /// Block index.
        index: usize,
        /// Block type (e.g., "text", "tool_use").
        content_type: String,
        /// Tool use ID (only for tool_use blocks).
        tool_id: Option<String>,
        /// Tool name (only for tool_use blocks).
        tool_name: Option<String>,
    },
    /// Incremental content within a block.
    ContentBlockDelta {
        /// Block index.
        index: usize,
        /// The delta content.
        delta: ContentDelta,
    },
    /// End of a content block.
    ContentBlockStop {
        /// Block index.
        index: usize,
    },
    /// Final message-level metadata.
    MessageDelta {
        /// Reason the message stopped.
        stop_reason: Option<String>,
        /// Token usage info.
        usage: Usage,
    },
    /// End of the message stream.
    MessageStop,
    /// Keep-alive ping.
    Ping,
    /// An error event.
    Error(String),
}

/// Incremental content delta within a streaming block.
#[derive(Debug, Clone)]
pub enum ContentDelta {
    /// Incremental text.
    TextDelta(String),
    /// Incremental JSON for tool input.
    InputJsonDelta(String),
    /// Incremental thinking text.
    ThinkingDelta(String),
    /// Incremental signature.
    SignatureDelta(String),
}

/// Token usage statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Usage {
    /// Input tokens consumed.
    #[serde(default)]
    pub input_tokens: u64,
    /// Output tokens generated.
    #[serde(default)]
    pub output_tokens: u64,
    /// Tokens read from cache.
    #[serde(default)]
    pub cache_creation_input_tokens: Option<u64>,
    /// Tokens written to cache.
    #[serde(default)]
    pub cache_read_input_tokens: Option<u64>,
}

/// Parse a single SSE event from its event type and JSON data.
///
/// Returns `None` if the event type is unknown or unparseable.
#[must_use]
pub fn parse_sse_event(event_type: &str, data: &str) -> Option<StreamEvent> {
    match event_type {
        "message_start" => {
            let v: serde_json::Value = serde_json::from_str(data).ok()?;
            let msg = v.get("message")?;
            Some(StreamEvent::MessageStart {
                id: msg["id"].as_str().unwrap_or_default().to_string(),
                model: msg["model"].as_str().unwrap_or_default().to_string(),
            })
        }
        "content_block_start" => {
            let v: serde_json::Value = serde_json::from_str(data).ok()?;
            let index = v["index"].as_u64().unwrap_or(0) as usize;
            let cb = v.get("content_block")?;
            let content_type = cb["type"].as_str().unwrap_or("text").to_string();
            let tool_id = cb["id"].as_str().map(String::from);
            let tool_name = cb["name"].as_str().map(String::from);
            Some(StreamEvent::ContentBlockStart {
                index,
                content_type,
                tool_id,
                tool_name,
            })
        }
        "content_block_delta" => {
            let v: serde_json::Value = serde_json::from_str(data).ok()?;
            let index = v["index"].as_u64().unwrap_or(0) as usize;
            let delta = v.get("delta")?;
            let delta_type = delta["type"].as_str().unwrap_or("");
            let content_delta = match delta_type {
                "text_delta" => {
                    ContentDelta::TextDelta(delta["text"].as_str().unwrap_or("").to_string())
                }
                "input_json_delta" => ContentDelta::InputJsonDelta(
                    delta["partial_json"].as_str().unwrap_or("").to_string(),
                ),
                "thinking_delta" => {
                    ContentDelta::ThinkingDelta(delta["thinking"].as_str().unwrap_or("").to_string())
                }
                "signature_delta" => ContentDelta::SignatureDelta(
                    delta["signature"].as_str().unwrap_or("").to_string(),
                ),
                _ => return None,
            };
            Some(StreamEvent::ContentBlockDelta {
                index,
                delta: content_delta,
            })
        }
        "content_block_stop" => {
            let v: serde_json::Value = serde_json::from_str(data).ok()?;
            let index = v["index"].as_u64().unwrap_or(0) as usize;
            Some(StreamEvent::ContentBlockStop { index })
        }
        "message_delta" => {
            let v: serde_json::Value = serde_json::from_str(data).ok()?;
            let delta = v.get("delta")?;
            let stop_reason = delta["stop_reason"].as_str().map(String::from);
            let usage = v
                .get("usage")
                .and_then(|u| serde_json::from_value(u.clone()).ok())
                .unwrap_or_default();
            Some(StreamEvent::MessageDelta { stop_reason, usage })
        }
        "message_stop" => Some(StreamEvent::MessageStop),
        "ping" => Some(StreamEvent::Ping),
        "error" => {
            let v: serde_json::Value = serde_json::from_str(data).ok()?;
            let msg = v["error"]["message"]
                .as_str()
                .or_else(|| v["message"].as_str())
                .unwrap_or("unknown error");
            Some(StreamEvent::Error(msg.to_string()))
        }
        _ => None,
    }
}

/// Parse raw SSE text lines into events.
///
/// SSE format: `event: <type>\ndata: <json>\n\n`
pub fn parse_sse_lines(text: &str) -> Vec<StreamEvent> {
    let mut events = Vec::new();
    let mut current_event = String::new();
    let mut current_data = String::new();

    for line in text.lines() {
        if let Some(ev) = line.strip_prefix("event: ") {
            current_event = ev.trim().to_string();
        } else if let Some(d) = line.strip_prefix("data: ") {
            current_data = d.to_string();
        } else if line.is_empty() && !current_event.is_empty() {
            if let Some(ev) = parse_sse_event(&current_event, &current_data) {
                events.push(ev);
            }
            current_event.clear();
            current_data.clear();
        }
    }
    // Handle trailing event without final blank line
    if !current_event.is_empty() {
        if let Some(ev) = parse_sse_event(&current_event, &current_data) {
            events.push(ev);
        }
    }

    events
}

/// Convert an HTTP response into a stream of `StreamEvent`s.
pub fn parse_sse_stream(
    response: reqwest::Response,
) -> impl Stream<Item = Result<StreamEvent, ApiError>> {
    let byte_stream = response.bytes_stream();

    futures_util::stream::unfold(
        (byte_stream, String::new()),
        |(mut stream, mut buffer)| async move {
            loop {
                // Check for complete events in buffer
                if let Some(pos) = buffer.find("\n\n") {
                    let chunk = buffer[..pos].to_string();
                    buffer = buffer[pos + 2..].to_string();

                    let mut event_type = String::new();
                    let mut data = String::new();
                    for line in chunk.lines() {
                        if let Some(rest) = line.strip_prefix("event: ") {
                            event_type = rest.trim().to_string();
                        } else if let Some(rest) = line.strip_prefix("data: ") {
                            data = rest.to_string();
                        }
                    }

                    if !event_type.is_empty() {
                        if let Some(ev) = parse_sse_event(&event_type, &data) {
                            return Some((Ok(ev), (stream, buffer)));
                        }
                    }
                    continue;
                }

                // Need more data from network
                match stream.next().await {
                    Some(Ok(bytes)) => {
                        let text = String::from_utf8_lossy(&bytes);
                        buffer.push_str(&text);
                    }
                    Some(Err(e)) => {
                        return Some((Err(ApiError::NetworkError(e)), (stream, buffer)));
                    }
                    None => {
                        // Stream ended; process any remaining data
                        if !buffer.trim().is_empty() {
                            let mut event_type = String::new();
                            let mut data = String::new();
                            for line in buffer.lines() {
                                if let Some(rest) = line.strip_prefix("event: ") {
                                    event_type = rest.trim().to_string();
                                } else if let Some(rest) = line.strip_prefix("data: ") {
                                    data = rest.to_string();
                                }
                            }
                            buffer.clear();
                            if !event_type.is_empty() {
                                if let Some(ev) = parse_sse_event(&event_type, &data) {
                                    return Some((Ok(ev), (stream, buffer)));
                                }
                            }
                        }
                        return None;
                    }
                }
            }
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_message_start() {
        let data = r#"{"message":{"id":"msg_1","type":"message","role":"assistant","model":"claude-3","content":[]}}"#;
        let ev = parse_sse_event("message_start", data).unwrap();
        if let StreamEvent::MessageStart { id, model } = ev {
            assert_eq!(id, "msg_1");
            assert_eq!(model, "claude-3");
        } else {
            panic!("expected MessageStart");
        }
    }

    #[test]
    fn test_parse_content_block_start_text() {
        let data = r#"{"index":0,"content_block":{"type":"text","text":""}}"#;
        let ev = parse_sse_event("content_block_start", data).unwrap();
        if let StreamEvent::ContentBlockStart {
            index,
            content_type,
            tool_id,
            tool_name,
        } = ev
        {
            assert_eq!(index, 0);
            assert_eq!(content_type, "text");
            assert!(tool_id.is_none());
            assert!(tool_name.is_none());
        } else {
            panic!("expected ContentBlockStart");
        }
    }

    #[test]
    fn test_parse_content_block_start_tool_use() {
        let data = r#"{"index":1,"content_block":{"type":"tool_use","id":"tu_1","name":"Bash","input":{}}}"#;
        let ev = parse_sse_event("content_block_start", data).unwrap();
        if let StreamEvent::ContentBlockStart {
            index,
            content_type,
            tool_id,
            tool_name,
        } = ev
        {
            assert_eq!(index, 1);
            assert_eq!(content_type, "tool_use");
            assert_eq!(tool_id.as_deref(), Some("tu_1"));
            assert_eq!(tool_name.as_deref(), Some("Bash"));
        } else {
            panic!("expected ContentBlockStart");
        }
    }

    #[test]
    fn test_parse_text_delta() {
        let data = r#"{"index":0,"delta":{"type":"text_delta","text":"Hello"}}"#;
        let ev = parse_sse_event("content_block_delta", data).unwrap();
        if let StreamEvent::ContentBlockDelta { index, delta } = ev {
            assert_eq!(index, 0);
            if let ContentDelta::TextDelta(text) = delta {
                assert_eq!(text, "Hello");
            } else {
                panic!("expected TextDelta");
            }
        } else {
            panic!("expected ContentBlockDelta");
        }
    }

    #[test]
    fn test_parse_input_json_delta() {
        let data =
            r#"{"index":1,"delta":{"type":"input_json_delta","partial_json":"{\"cmd\":\"ls\"}"}}"#;
        let ev = parse_sse_event("content_block_delta", data).unwrap();
        if let StreamEvent::ContentBlockDelta { delta, .. } = ev {
            if let ContentDelta::InputJsonDelta(json) = delta {
                assert!(json.contains("cmd"));
            } else {
                panic!("expected InputJsonDelta");
            }
        } else {
            panic!("expected ContentBlockDelta");
        }
    }

    #[test]
    fn test_parse_content_block_stop() {
        let data = r#"{"index":0}"#;
        let ev = parse_sse_event("content_block_stop", data).unwrap();
        if let StreamEvent::ContentBlockStop { index } = ev {
            assert_eq!(index, 0);
        } else {
            panic!("expected ContentBlockStop");
        }
    }

    #[test]
    fn test_parse_message_delta() {
        let data = r#"{"delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":42}}"#;
        let ev = parse_sse_event("message_delta", data).unwrap();
        if let StreamEvent::MessageDelta { stop_reason, usage } = ev {
            assert_eq!(stop_reason.as_deref(), Some("end_turn"));
            assert_eq!(usage.output_tokens, 42);
        } else {
            panic!("expected MessageDelta");
        }
    }

    #[test]
    fn test_parse_message_stop() {
        let ev = parse_sse_event("message_stop", "{}").unwrap();
        assert!(matches!(ev, StreamEvent::MessageStop));
    }

    #[test]
    fn test_parse_ping() {
        let ev = parse_sse_event("ping", "{}").unwrap();
        assert!(matches!(ev, StreamEvent::Ping));
    }

    #[test]
    fn test_parse_error_event() {
        let data = r#"{"error":{"message":"overloaded"}}"#;
        let ev = parse_sse_event("error", data).unwrap();
        if let StreamEvent::Error(msg) = ev {
            assert_eq!(msg, "overloaded");
        } else {
            panic!("expected Error");
        }
    }

    #[test]
    fn test_parse_unknown_event() {
        assert!(parse_sse_event("unknown_type", "{}").is_none());
    }

    #[test]
    fn test_parse_sse_lines_multiple() {
        let text = "\
event: message_start\n\
data: {\"message\":{\"id\":\"m1\",\"model\":\"c3\",\"role\":\"assistant\",\"content\":[]}}\n\
\n\
event: content_block_start\n\
data: {\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\
\n\
event: content_block_delta\n\
data: {\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Hi\"}}\n\
\n\
event: message_stop\n\
data: {}\n\
\n";
        let events = parse_sse_lines(text);
        assert_eq!(events.len(), 4);
        assert!(matches!(events[0], StreamEvent::MessageStart { .. }));
        assert!(matches!(events[1], StreamEvent::ContentBlockStart { .. }));
        assert!(matches!(events[2], StreamEvent::ContentBlockDelta { .. }));
        assert!(matches!(events[3], StreamEvent::MessageStop));
    }

    #[test]
    fn test_usage_default() {
        let u = Usage::default();
        assert_eq!(u.input_tokens, 0);
        assert_eq!(u.output_tokens, 0);
        assert!(u.cache_creation_input_tokens.is_none());
        assert!(u.cache_read_input_tokens.is_none());
    }

    #[test]
    fn test_usage_serde_roundtrip() {
        let u = Usage {
            input_tokens: 100,
            output_tokens: 50,
            cache_creation_input_tokens: Some(10),
            cache_read_input_tokens: None,
        };
        let json = serde_json::to_string(&u).unwrap();
        let parsed: Usage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.input_tokens, 100);
        assert_eq!(parsed.output_tokens, 50);
        assert_eq!(parsed.cache_creation_input_tokens, Some(10));
    }

    #[test]
    fn test_parse_thinking_delta() {
        let data = r#"{"index":0,"delta":{"type":"thinking_delta","thinking":"Let me think..."}}"#;
        let ev = parse_sse_event("content_block_delta", data).unwrap();
        if let StreamEvent::ContentBlockDelta { delta, .. } = ev {
            assert!(matches!(delta, ContentDelta::ThinkingDelta(t) if t == "Let me think..."));
        } else {
            panic!("expected ContentBlockDelta");
        }
    }
}
