//! Message normalization for the Anthropic Messages API.

use claude_core::message::{Message, Role};

/// Normalize a message list to satisfy API constraints.
///
/// Ensures:
/// 1. The list starts with a user message.
/// 2. Messages strictly alternate between user and assistant.
/// 3. Consecutive same-role messages are merged.
#[must_use]
pub fn normalize_messages(messages: &[Message]) -> Vec<Message> {
    if messages.is_empty() {
        return Vec::new();
    }

    let mut result: Vec<Message> = Vec::with_capacity(messages.len());

    for msg in messages {
        if let Some(last) = result.last_mut() {
            if last.role == msg.role {
                // Merge consecutive same-role messages
                last.content.extend(msg.content.clone());
            } else {
                result.push(msg.clone());
            }
        } else {
            // First message must be user
            if msg.role == Role::User {
                result.push(msg.clone());
            } else {
                // Insert a synthetic user message before the assistant message
                result.push(Message::user("[conversation continues]"));
                result.push(msg.clone());
            }
        }
    }

    // Ensure we start with user
    if let Some(first) = result.first() {
        if first.role != Role::User {
            result.insert(0, Message::user("[conversation continues]"));
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use claude_core::message::ContentBlock;

    #[test]
    fn test_empty_input() {
        let result = normalize_messages(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_single_user_message() {
        let msgs = vec![Message::user("hello")];
        let result = normalize_messages(&msgs);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].role, Role::User);
    }

    #[test]
    fn test_already_alternating() {
        let msgs = vec![Message::user("hi"), Message::assistant("hello")];
        let result = normalize_messages(&msgs);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].role, Role::User);
        assert_eq!(result[1].role, Role::Assistant);
    }

    #[test]
    fn test_merge_consecutive_user() {
        let msgs = vec![
            Message::user("hello"),
            Message::user("world"),
            Message::assistant("hi"),
        ];
        let result = normalize_messages(&msgs);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].role, Role::User);
        assert_eq!(result[0].content.len(), 2);
    }

    #[test]
    fn test_assistant_first_gets_synthetic_user() {
        let msgs = vec![Message::assistant("hi there")];
        let result = normalize_messages(&msgs);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].role, Role::User);
        assert_eq!(result[1].role, Role::Assistant);
    }

    #[test]
    fn test_merge_consecutive_assistant() {
        let msgs = vec![
            Message::user("go"),
            Message::assistant("part 1"),
            Message::assistant("part 2"),
        ];
        let result = normalize_messages(&msgs);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].role, Role::User);
        assert_eq!(result[1].role, Role::Assistant);
        assert_eq!(result[1].content.len(), 2);
    }

    #[test]
    fn test_tool_result_messages_merged() {
        let msgs = vec![
            Message::user("run tool"),
            Message {
                role: Role::Assistant,
                content: vec![ContentBlock::ToolUse {
                    id: "tu_1".into(),
                    name: "Bash".into(),
                    input: serde_json::json!({"command": "ls"}),
                }],
                cache_control: None,
            },
            Message {
                role: Role::User,
                content: vec![ContentBlock::ToolResult {
                    tool_use_id: "tu_1".into(),
                    content: vec![],
                    is_error: None,
                }],
                cache_control: None,
            },
        ];
        let result = normalize_messages(&msgs);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].role, Role::User);
        assert_eq!(result[1].role, Role::Assistant);
        assert_eq!(result[2].role, Role::User);
    }
}
