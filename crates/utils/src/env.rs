//! Environment variable helpers.

/// Returns `true` when running in a CI environment.
#[must_use]
pub fn is_ci() -> bool {
    std::env::var_os("CI").is_some()
        || std::env::var_os("GITHUB_ACTIONS").is_some()
        || std::env::var_os("JENKINS_URL").is_some()
        || std::env::var_os("CIRCLECI").is_some()
        || std::env::var_os("TRAVIS").is_some()
        || std::env::var_os("GITLAB_CI").is_some()
}

/// Read an environment variable, falling back to `default` if unset or empty.
#[must_use]
pub fn get_env_or(key: &str, default: &str) -> String {
    std::env::var(key)
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| default.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_env_or_default() {
        let val = get_env_or("__CLAUDE_UTILS_TEST_MISSING__", "fallback");
        assert_eq!(val, "fallback");
    }

    #[test]
    fn test_get_env_or_existing() {
        // SAFETY: single-threaded test; no other threads reading this var.
        unsafe { std::env::set_var("__CLAUDE_UTILS_TEST_KEY__", "hello") };
        let val = get_env_or("__CLAUDE_UTILS_TEST_KEY__", "fallback");
        assert_eq!(val, "hello");
        unsafe { std::env::remove_var("__CLAUDE_UTILS_TEST_KEY__") };
    }
}
