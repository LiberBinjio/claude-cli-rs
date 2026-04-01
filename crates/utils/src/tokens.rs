//! Rough token-count estimation and budget truncation.

/// Estimate token count: `chars / 3.5` rounded up.
#[inline]
#[must_use]
pub fn estimate_token_count(text: &str) -> u64 {
    let chars = text.chars().count() as f64;
    (chars / 3.5).ceil() as u64
}

/// Truncate `text` so its estimated token count is at most `max_tokens`.
///
/// Truncation is done at a character boundary (not mid-char).
#[must_use]
pub fn truncate_to_token_budget(text: &str, max_tokens: u64) -> String {
    let max_chars = (max_tokens as f64 * 3.5).floor() as usize;
    if text.chars().count() <= max_chars {
        return text.to_owned();
    }
    text.chars().take(max_chars).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_token_count() {
        // 7 chars → 7/3.5 = 2.0 → 2
        assert_eq!(estimate_token_count("abcdefg"), 2);
        // 8 chars → 8/3.5 ≈ 2.286 → 3
        assert_eq!(estimate_token_count("abcdefgh"), 3);
    }

    #[test]
    fn test_truncate_no_op() {
        let s = "short";
        let result = truncate_to_token_budget(s, 100);
        assert_eq!(result, s);
    }

    #[test]
    fn test_truncate_cuts() {
        let long = "a".repeat(1000);
        let result = truncate_to_token_budget(&long, 10);
        // 10 tokens × 3.5 = 35 chars
        assert_eq!(result.len(), 35);
    }
}
