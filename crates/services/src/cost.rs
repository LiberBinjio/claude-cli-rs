use claude_api::Usage;

/// Tracks cumulative token usage and computes estimated cost.
#[derive(Debug, Clone, Default)]
pub struct CostTracker {
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cache_read_tokens: u64,
    pub total_cache_write_tokens: u64,
}

impl CostTracker {
    /// Create a zeroed-out tracker.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Accumulate usage from an API response.
    pub fn add_usage(&mut self, usage: &Usage) {
        self.total_input_tokens += usage.input_tokens;
        self.total_output_tokens += usage.output_tokens;
        self.total_cache_read_tokens += usage.cache_read_input_tokens.unwrap_or(0);
        self.total_cache_write_tokens += usage.cache_creation_input_tokens.unwrap_or(0);
    }

    /// Estimate total cost in USD based on model pricing.
    ///
    /// Pricing (per million tokens) as of 2025:
    /// - claude-sonnet-4: input $3, output $15, cache-read $0.30, cache-write $3.75
    /// - claude-haiku-4:  input $0.80, output $4, cache-read $0.08, cache-write $1
    /// - claude-opus-4:   input $15, output $75, cache-read $1.50, cache-write $18.75
    pub fn total_cost_usd(&self, model: &str) -> f64 {
        let (input_rate, output_rate, cache_read_rate, cache_write_rate) =
            if model.contains("opus") {
                (15.0, 75.0, 1.50, 18.75)
            } else if model.contains("haiku") {
                (0.80, 4.0, 0.08, 1.0)
            } else {
                // Default to Sonnet pricing
                (3.0, 15.0, 0.30, 3.75)
            };

        let m = 1_000_000.0;
        (self.total_input_tokens as f64 / m) * input_rate
            + (self.total_output_tokens as f64 / m) * output_rate
            + (self.total_cache_read_tokens as f64 / m) * cache_read_rate
            + (self.total_cache_write_tokens as f64 / m) * cache_write_rate
    }

    /// Format a human-readable cost summary.
    pub fn summary(&self, model: &str) -> String {
        let cost = self.total_cost_usd(model);
        format!(
            "Token Usage:\n  Input:       {:>8}\n  Output:      {:>8}\n  Cache read:  {:>8}\n  Cache write: {:>8}\n  Total cost:  ${:.4}",
            self.total_input_tokens,
            self.total_output_tokens,
            self.total_cache_read_tokens,
            self.total_cache_write_tokens,
            cost,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_usage() {
        let mut tracker = CostTracker::new();
        let usage = Usage {
            input_tokens: 1000,
            output_tokens: 500,
            cache_read_input_tokens: Some(200),
            cache_creation_input_tokens: Some(100),
        };
        tracker.add_usage(&usage);
        assert_eq!(tracker.total_input_tokens, 1000);
        assert_eq!(tracker.total_output_tokens, 500);
        assert_eq!(tracker.total_cache_read_tokens, 200);
        assert_eq!(tracker.total_cache_write_tokens, 100);
    }

    #[test]
    fn test_add_usage_accumulates() {
        let mut tracker = CostTracker::new();
        let usage = Usage {
            input_tokens: 100,
            output_tokens: 50,
            cache_read_input_tokens: None,
            cache_creation_input_tokens: None,
        };
        tracker.add_usage(&usage);
        tracker.add_usage(&usage);
        assert_eq!(tracker.total_input_tokens, 200);
        assert_eq!(tracker.total_output_tokens, 100);
    }

    #[test]
    fn test_cost_sonnet() {
        let mut tracker = CostTracker::new();
        tracker.total_input_tokens = 1_000_000;
        tracker.total_output_tokens = 1_000_000;
        let cost = tracker.total_cost_usd("claude-sonnet-4-20250514");
        // $3 + $15 = $18
        assert!((cost - 18.0).abs() < 0.01);
    }

    #[test]
    fn test_cost_haiku() {
        let mut tracker = CostTracker::new();
        tracker.total_input_tokens = 1_000_000;
        tracker.total_output_tokens = 1_000_000;
        let cost = tracker.total_cost_usd("claude-haiku-4-20250414");
        // $0.80 + $4 = $4.80
        assert!((cost - 4.80).abs() < 0.01);
    }

    #[test]
    fn test_cost_opus() {
        let mut tracker = CostTracker::new();
        tracker.total_input_tokens = 1_000_000;
        tracker.total_output_tokens = 1_000_000;
        let cost = tracker.total_cost_usd("claude-opus-4");
        // $15 + $75 = $90
        assert!((cost - 90.0).abs() < 0.01);
    }

    #[test]
    fn test_summary_format() {
        let tracker = CostTracker {
            total_input_tokens: 1500,
            total_output_tokens: 300,
            total_cache_read_tokens: 0,
            total_cache_write_tokens: 0,
        };
        let summary = tracker.summary("claude-sonnet-4-20250514");
        assert!(summary.contains("1500"));
        assert!(summary.contains("300"));
        assert!(summary.contains("$"));
    }

    #[test]
    fn test_cost_with_cache_tokens() {
        let mut tracker = CostTracker::new();
        tracker.total_input_tokens = 1_000_000;
        tracker.total_cache_read_tokens = 1_000_000;
        tracker.total_cache_write_tokens = 1_000_000;
        let cost = tracker.total_cost_usd("claude-sonnet-4-20250514");
        // $3 (input) + $0 (output) + $0.30 (cache_read) + $3.75 (cache_write) = $7.05
        assert!((cost - 7.05).abs() < 0.01);
    }

    #[test]
    fn test_cost_zero_tokens() {
        let tracker = CostTracker::new();
        let cost = tracker.total_cost_usd("claude-sonnet-4-20250514");
        assert!((cost - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_default_is_zeroed() {
        let tracker = CostTracker::default();
        assert_eq!(tracker.total_input_tokens, 0);
        assert_eq!(tracker.total_output_tokens, 0);
        assert_eq!(tracker.total_cache_read_tokens, 0);
        assert_eq!(tracker.total_cache_write_tokens, 0);
    }
}
