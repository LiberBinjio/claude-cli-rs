//! Context compaction strategy — decide when and how to compact.

/// Configuration for the compaction service.
#[derive(Debug, Clone)]
pub struct CompactService {
    /// Threshold (estimated tokens) above which compaction triggers.
    pub threshold_tokens: u64,
    /// Number of most-recent messages to preserve uncompacted.
    pub keep_recent: usize,
}

impl Default for CompactService {
    fn default() -> Self {
        Self {
            threshold_tokens: 100_000,
            keep_recent: 10,
        }
    }
}

impl CompactService {
    /// Create with custom thresholds.
    #[must_use]
    pub fn new(threshold: u64, keep_recent: usize) -> Self {
        Self {
            threshold_tokens: threshold,
            keep_recent,
        }
    }

    /// Returns `true` when `total_tokens` exceeds the configured threshold.
    #[inline]
    #[must_use]
    pub fn should_compact(&self, total_tokens: u64) -> bool {
        total_tokens > self.threshold_tokens
    }

    /// System prompt instructing the model to summarise the conversation.
    #[must_use]
    pub fn compact_prompt(&self) -> &'static str {
        "Please provide a concise summary of our conversation so far, \
         preserving all important context, decisions, code changes, \
         and file paths mentioned. This summary will replace the \
         conversation history to save tokens."
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_compact() {
        let svc = CompactService::default();
        assert!(!svc.should_compact(50_000));
        assert!(svc.should_compact(150_000));
    }

    #[test]
    fn test_custom_threshold() {
        let svc = CompactService::new(1000, 5);
        assert!(svc.should_compact(1001));
        assert!(!svc.should_compact(999));
    }

    #[test]
    fn test_compact_prompt_not_empty() {
        let svc = CompactService::default();
        assert!(!svc.compact_prompt().is_empty());
    }
}
