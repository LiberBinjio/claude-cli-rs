//! Usage tips — randomly selected hints for the user.

/// All available tips.
const TIPS: &[&str] = &[
    "Use /compact to compress conversation history and save tokens.",
    "Use /model to switch between different Claude models.",
    "Use Ctrl+C to cancel a running operation.",
    "Use /help to see all available commands.",
    "Use /cost to check your token usage.",
    "Add files to context with drag-and-drop or /add.",
    "Use /session to manage and resume past conversations.",
    "Press Tab to autocomplete slash commands.",
];

/// Return a pseudo-random tip.
#[must_use]
pub fn random_tip() -> &'static str {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as usize;
    TIPS[nanos % TIPS.len()]
}

/// Return all tips.
#[must_use]
pub fn all_tips() -> &'static [&'static str] {
    TIPS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_tip_not_empty() {
        assert!(!random_tip().is_empty());
    }

    #[test]
    fn test_all_tips_count() {
        assert!(all_tips().len() >= 5);
    }

    #[test]
    fn test_tip_is_in_list() {
        let tip = random_tip();
        assert!(all_tips().contains(&tip));
    }
}
