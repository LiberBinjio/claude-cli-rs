//! Context compaction — truncate old messages when conversation exceeds budget.

use claude_core::message::{ContentBlock, Message, Role};

/// Configuration for context compaction.
#[derive(Debug, Clone)]
pub struct CompactConfig {
    /// Token count threshold to trigger compaction.
    pub threshold: u64,
    /// Number of recent messages to preserve.
    pub keep_recent: usize,
}

impl Default for CompactConfig {
    fn default() -> Self {
        Self {
            threshold: 100_000,
            keep_recent: 10,
        }
    }
}

/// Estimate the total token count of a message list.
///
/// Uses a rough heuristic: ~4 characters per token.
#[must_use]
pub fn estimate_messages_tokens(messages: &[Message]) -> u64 {
    messages
        .iter()
        .map(|msg| {
            let chars: u64 = msg
                .content
                .iter()
                .map(|block| match block {
                    ContentBlock::Text { text } => text.len() as u64,
                    ContentBlock::ToolUse { input, .. } => input.to_string().len() as u64,
                    ContentBlock::ToolResult { content, .. } => content
                        .iter()
                        .map(|c| c.text.as_deref().unwrap_or("").len() as u64)
                        .sum(),
                    ContentBlock::Image { .. } => 200, // images ~200 tokens estimate
                })
                .sum::<u64>();
            // ~4 chars per token
            chars / 4 + 1
        })
        .sum()
}

/// Check whether compaction should be triggered.
#[must_use]
pub fn should_compact(messages: &[Message], config: &CompactConfig) -> bool {
    estimate_messages_tokens(messages) > config.threshold
}

/// Compact messages by summarizing older ones and keeping recent ones.
///
/// Returns a new message list with a summary of older messages prepended
/// to the most recent `keep_recent` messages.
#[must_use]
pub fn compact_messages(messages: &[Message], config: &CompactConfig) -> Vec<Message> {
    if messages.len() <= config.keep_recent {
        return messages.to_vec();
    }

    let split_point = messages.len().saturating_sub(config.keep_recent);
    let (old, recent) = messages.split_at(split_point);

    // Generate summary of old messages
    let summary = generate_summary(old);

    let mut result = Vec::with_capacity(1 + recent.len());
    result.push(Message::user(summary));
    result.extend_from_slice(recent);
    result
}

/// Generate a text summary from a list of old messages.
fn generate_summary(messages: &[Message]) -> String {
    let mut summary =
        String::from("[Conversation compacted. Summary of earlier messages:]\n");

    for msg in messages {
        let role_str = match msg.role {
            Role::User => "User",
            Role::Assistant => "Assistant",
        };
        let first_text = msg.content.iter().find_map(|b| {
            if let ContentBlock::Text { text } = b {
                let truncated: String = text.chars().take(200).collect();
                if !truncated.is_empty() {
                    Some(truncated)
                } else {
                    None
                }
            } else {
                None
            }
        });
        if let Some(text) = first_text {
            summary.push_str(&format!("{role_str}: {text}\n"));
        }
    }

    summary
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_short_messages() {
        let msgs = vec![Message::user("hello")];
        let tokens = estimate_messages_tokens(&msgs);
        assert!(tokens > 0);
        assert!(tokens < 100);
    }

    #[test]
    fn test_should_compact_under_threshold() {
        let msgs = vec![Message::user("hello")];
        assert!(!should_compact(&msgs, &CompactConfig::default()));
    }

    #[test]
    fn test_should_compact_over_threshold() {
        let config = CompactConfig {
            threshold: 1,
            keep_recent: 2,
        };
        let msgs = vec![Message::user("hello world")];
        assert!(should_compact(&msgs, &config));
    }

    #[test]
    fn test_compact_preserves_all_when_under_limit() {
        let config = CompactConfig {
            threshold: 0,
            keep_recent: 10,
        };
        let msgs = vec![Message::user("a"), Message::assistant("b")];
        let result = compact_messages(&msgs, &config);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_compact_reduces_old_messages() {
        let config = CompactConfig {
            threshold: 0,
            keep_recent: 2,
        };
        let msgs = vec![
            Message::user("old1"),
            Message::assistant("old2"),
            Message::user("recent1"),
            Message::assistant("recent2"),
        ];
        let result = compact_messages(&msgs, &config);
        // 1 summary + 2 recent = 3
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].role, Role::User);
        assert!(result[0].text().contains("[Conversation compacted"));
    }

    #[test]
    fn test_generate_summary_content() {
        let msgs = vec![
            Message::user("first question"),
            Message::assistant("first answer"),
        ];
        let summary = generate_summary(&msgs);
        assert!(summary.contains("User: first question"));
        assert!(summary.contains("Assistant: first answer"));
    }

    #[test]
    fn test_summary_truncates_long_text() {
        let long_text = "x".repeat(500);
        let msgs = vec![Message::user(long_text)];
        let summary = generate_summary(&msgs);
        // Should be truncated to ~200 chars + prefix
        assert!(summary.len() < 400);
    }
}
