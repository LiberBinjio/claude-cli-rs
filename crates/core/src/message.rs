//! Message types for the Anthropic Messages API.

use serde::{Deserialize, Serialize};

/// A single message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// The role of the message sender.
    pub role: Role,
    /// The content blocks of this message.
    pub content: Vec<ContentBlock>,
    /// Optional cache control directive.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControl>,
}

impl Message {
    /// Create a new user text message.
    #[must_use]
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: vec![ContentBlock::Text { text: text.into() }],
            cache_control: None,
        }
    }

    /// Create a new assistant text message.
    #[must_use]
    pub fn assistant(text: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: vec![ContentBlock::Text { text: text.into() }],
            cache_control: None,
        }
    }

    /// Extract all text content from this message.
    #[must_use]
    pub fn text(&self) -> String {
        self.content
            .iter()
            .filter_map(|block| {
                if let ContentBlock::Text { text } = block {
                    Some(text.as_str())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("")
    }
}

/// The role of a message sender.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// A human user.
    User,
    /// The AI assistant.
    Assistant,
}

/// A content block within a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// A text block.
    Text {
        /// The text content.
        text: String,
    },
    /// A tool use request from the assistant.
    ToolUse {
        /// Unique identifier for this tool use.
        id: String,
        /// The name of the tool to call.
        name: String,
        /// The input to the tool as JSON.
        input: serde_json::Value,
    },
    /// A tool result from the user.
    ToolResult {
        /// The ID of the tool use this is a result for.
        tool_use_id: String,
        /// The result content.
        content: Vec<ToolResultContent>,
        /// Whether this result is an error.
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
    },
    /// An image block.
    Image {
        /// The image source.
        source: ImageSource,
    },
}

/// Content within a tool result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultContent {
    /// The type of content (e.g., "text").
    #[serde(rename = "type")]
    pub content_type: String,
    /// Optional text content.
    pub text: Option<String>,
}

/// Source data for an image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSource {
    /// The source type (e.g., "base64").
    #[serde(rename = "type")]
    pub source_type: String,
    /// The MIME type (e.g., "image/png").
    pub media_type: String,
    /// The base64-encoded image data.
    pub data: String,
}

/// Cache control directive for prompt caching.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheControl {
    /// The cache type (e.g., "ephemeral").
    #[serde(rename = "type")]
    pub cache_type: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_user_constructor() {
        let msg = Message::user("hello");
        assert_eq!(msg.role, Role::User);
        assert_eq!(msg.text(), "hello");
        assert!(msg.cache_control.is_none());
    }

    #[test]
    fn test_message_assistant_constructor() {
        let msg = Message::assistant("world");
        assert_eq!(msg.role, Role::Assistant);
        assert_eq!(msg.text(), "world");
    }

    #[test]
    fn test_role_serde_roundtrip() {
        let json = serde_json::to_string(&Role::User).unwrap();
        assert_eq!(json, r#""user""#);
        let parsed: Role = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, Role::User);

        let json = serde_json::to_string(&Role::Assistant).unwrap();
        assert_eq!(json, r#""assistant""#);
    }

    #[test]
    fn test_content_block_text_serde() {
        let block = ContentBlock::Text {
            text: "hello".into(),
        };
        let json = serde_json::to_string(&block).unwrap();
        assert!(json.contains(r#""type":"text""#));
        assert!(json.contains(r#""text":"hello""#));

        let parsed: ContentBlock = serde_json::from_str(&json).unwrap();
        if let ContentBlock::Text { text } = parsed {
            assert_eq!(text, "hello");
        } else {
            panic!("expected Text block");
        }
    }

    #[test]
    fn test_content_block_tool_use_serde() {
        let block = ContentBlock::ToolUse {
            id: "tu_1".into(),
            name: "Bash".into(),
            input: serde_json::json!({"command": "ls"}),
        };
        let json = serde_json::to_string(&block).unwrap();
        assert!(json.contains(r#""type":"tool_use""#));

        let parsed: ContentBlock = serde_json::from_str(&json).unwrap();
        if let ContentBlock::ToolUse { id, name, input } = parsed {
            assert_eq!(id, "tu_1");
            assert_eq!(name, "Bash");
            assert_eq!(input["command"], "ls");
        } else {
            panic!("expected ToolUse block");
        }
    }

    #[test]
    fn test_content_block_tool_result_serde() {
        let block = ContentBlock::ToolResult {
            tool_use_id: "tu_1".into(),
            content: vec![ToolResultContent {
                content_type: "text".into(),
                text: Some("output".into()),
            }],
            is_error: Some(false),
        };
        let json = serde_json::to_string(&block).unwrap();
        assert!(json.contains(r#""type":"tool_result""#));

        let parsed: ContentBlock = serde_json::from_str(&json).unwrap();
        if let ContentBlock::ToolResult {
            tool_use_id,
            content,
            is_error,
        } = parsed
        {
            assert_eq!(tool_use_id, "tu_1");
            assert_eq!(content.len(), 1);
            assert_eq!(is_error, Some(false));
        } else {
            panic!("expected ToolResult block");
        }
    }

    #[test]
    fn test_message_serde_roundtrip() {
        let msg = Message {
            role: Role::User,
            content: vec![
                ContentBlock::Text {
                    text: "Hello".into(),
                },
                ContentBlock::Text {
                    text: " world".into(),
                },
            ],
            cache_control: None,
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.role, Role::User);
        assert_eq!(parsed.content.len(), 2);
        assert_eq!(parsed.text(), "Hello world");
    }

    #[test]
    fn test_cache_control_not_serialized_when_none() {
        let msg = Message::user("hi");
        let json = serde_json::to_string(&msg).unwrap();
        assert!(!json.contains("cache_control"));
    }

    #[test]
    fn test_image_source_serde() {
        let src = ImageSource {
            source_type: "base64".into(),
            media_type: "image/png".into(),
            data: "abc123".into(),
        };
        let json = serde_json::to_string(&src).unwrap();
        assert!(json.contains(r#""type":"base64""#));
        assert!(json.contains(r#""media_type":"image/png""#));
    }
}
