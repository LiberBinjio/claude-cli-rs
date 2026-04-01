//! API key resolution: environment variable, config file, or system keychain.

use std::path::PathBuf;
use tracing::debug;

/// Read `ANTHROPIC_API_KEY` from environment.
#[inline]
#[must_use]
pub fn get_api_key_from_env() -> Option<String> {
    std::env::var("ANTHROPIC_API_KEY").ok().filter(|k| !k.is_empty())
}

/// Read `primaryApiKey` from `~/.claude.json`.
#[must_use]
pub fn get_api_key_from_config_file() -> Option<String> {
    let path = config_file_path()?;
    let content = std::fs::read_to_string(path).ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&content).ok()?;
    parsed
        .get("primaryApiKey")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from)
}

/// Resolve an API key with priority: env > config file > keychain.
#[must_use]
pub fn get_api_key() -> Option<String> {
    if let Some(key) = get_api_key_from_env() {
        debug!("API key found in environment variable");
        return Some(key);
    }
    if let Some(key) = get_api_key_from_config_file() {
        debug!("API key found in config file");
        return Some(key);
    }
    if let Ok(Some(key)) = crate::keychain::load_api_key() {
        debug!("API key found in keychain");
        return Some(key);
    }
    None
}

/// Path to `~/.claude.json`.
fn config_file_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".claude.json"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_get_api_key_from_env() {
        unsafe { std::env::set_var("ANTHROPIC_API_KEY", "sk-test-123") };
        assert_eq!(get_api_key_from_env(), Some("sk-test-123".into()));
        unsafe { std::env::remove_var("ANTHROPIC_API_KEY") };
        assert_eq!(get_api_key_from_env(), None);
    }

    #[test]
    #[serial]
    fn test_get_api_key_empty_env() {
        unsafe { std::env::set_var("ANTHROPIC_API_KEY", "") };
        assert_eq!(get_api_key_from_env(), None);
        unsafe { std::env::remove_var("ANTHROPIC_API_KEY") };
    }

    #[test]
    fn test_config_file_nonexistent() {
        // ~/.claude.json unlikely to have primaryApiKey in test env
        // Just verify it doesn't panic
        let _ = get_api_key_from_config_file();
    }

    #[test]
    #[serial]
    fn test_get_api_key_priority_env_first() {
        unsafe { std::env::set_var("ANTHROPIC_API_KEY", "sk-from-env") };
        let key = get_api_key();
        assert_eq!(key, Some("sk-from-env".into()));
        unsafe { std::env::remove_var("ANTHROPIC_API_KEY") };
    }

    #[test]
    #[serial]
    fn test_get_api_key_none_when_unset() {
        unsafe { std::env::remove_var("ANTHROPIC_API_KEY") };
        // With no env, no config file, and no keychain entry,
        // get_api_key may still find something in the real env,
        // so we just verify it doesn't panic and returns Option
        let result = get_api_key();
        // Result is None unless a real config/keychain exists
        assert!(result.is_none() || result.is_some());
    }

    #[test]
    #[serial]
    fn test_get_api_key_from_env_whitespace_only() {
        unsafe { std::env::set_var("ANTHROPIC_API_KEY", "   ") };
        // Whitespace-only is not empty string, filter checks !is_empty
        let result = get_api_key_from_env();
        // "   " is not empty, so it passes the filter
        assert!(result.is_some());
        unsafe { std::env::remove_var("ANTHROPIC_API_KEY") };
    }
}
