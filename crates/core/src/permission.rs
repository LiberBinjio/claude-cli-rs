//! Permission modes, rules, and decision logic.

use serde::{Deserialize, Serialize};

/// The active permission mode controlling tool access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PermissionMode {
    /// Default mode: read-only allowed, writes ask, explicit rules override.
    #[default]
    Default,
    /// Plan mode: only read-only tools allowed.
    Plan,
    /// Auto-edit mode: file edits allowed without asking.
    AutoEdit,
    /// Full auto mode: all tools allowed.
    FullAuto,
    /// Bypass all permission checks (dangerous).
    BypassPermissions,
}

/// A rule granting or denying access to a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRule {
    /// Tool name this rule applies to, or `"*"` for all tools.
    pub tool_name: String,
    /// Whether access is allowed or denied.
    pub allow: bool,
    /// Optional path glob pattern to match.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path_pattern: Option<String>,
    /// Optional command pattern to match.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command_pattern: Option<String>,
}

/// The outcome of a permission check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionDecision {
    /// Access is allowed.
    Allow,
    /// Access is denied.
    Deny,
    /// User must be asked.
    Ask,
}

/// Check whether a tool should be allowed, denied, or asked about.
///
/// Logic by mode:
/// - `BypassPermissions`: always allow
/// - `Plan`: allow read-only, deny writes
/// - `FullAuto`: always allow
/// - `AutoEdit`: allow read-only and file edit tools, ask otherwise
/// - `Default`: check explicit rules first, then allow read-only, ask for writes
#[must_use]
pub fn check_permission(
    mode: PermissionMode,
    tool_name: &str,
    is_read_only: bool,
    rules: &[PermissionRule],
) -> PermissionDecision {
    match mode {
        PermissionMode::BypassPermissions => PermissionDecision::Allow,
        PermissionMode::Plan => {
            if is_read_only {
                PermissionDecision::Allow
            } else {
                PermissionDecision::Deny
            }
        }
        PermissionMode::FullAuto => PermissionDecision::Allow,
        PermissionMode::AutoEdit => {
            if is_read_only || tool_name == "FileEdit" || tool_name == "FileWrite" {
                PermissionDecision::Allow
            } else {
                PermissionDecision::Ask
            }
        }
        PermissionMode::Default => {
            for rule in rules {
                if rule.tool_name == tool_name || rule.tool_name == "*" {
                    return if rule.allow {
                        PermissionDecision::Allow
                    } else {
                        PermissionDecision::Deny
                    };
                }
            }
            if is_read_only {
                PermissionDecision::Allow
            } else {
                PermissionDecision::Ask
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bypass_always_allows() {
        assert_eq!(
            check_permission(PermissionMode::BypassPermissions, "Bash", false, &[]),
            PermissionDecision::Allow
        );
    }

    #[test]
    fn test_plan_allows_readonly() {
        assert_eq!(
            check_permission(PermissionMode::Plan, "Read", true, &[]),
            PermissionDecision::Allow
        );
    }

    #[test]
    fn test_plan_denies_writes() {
        assert_eq!(
            check_permission(PermissionMode::Plan, "Bash", false, &[]),
            PermissionDecision::Deny
        );
    }

    #[test]
    fn test_fullauto_allows_all() {
        assert_eq!(
            check_permission(PermissionMode::FullAuto, "Bash", false, &[]),
            PermissionDecision::Allow
        );
    }

    #[test]
    fn test_autoedit_allows_file_write() {
        assert_eq!(
            check_permission(PermissionMode::AutoEdit, "FileEdit", false, &[]),
            PermissionDecision::Allow
        );
        assert_eq!(
            check_permission(PermissionMode::AutoEdit, "FileWrite", false, &[]),
            PermissionDecision::Allow
        );
    }

    #[test]
    fn test_autoedit_asks_for_bash() {
        assert_eq!(
            check_permission(PermissionMode::AutoEdit, "Bash", false, &[]),
            PermissionDecision::Ask
        );
    }

    #[test]
    fn test_default_allows_readonly() {
        assert_eq!(
            check_permission(PermissionMode::Default, "Read", true, &[]),
            PermissionDecision::Allow
        );
    }

    #[test]
    fn test_default_asks_for_write() {
        assert_eq!(
            check_permission(PermissionMode::Default, "Bash", false, &[]),
            PermissionDecision::Ask
        );
    }

    #[test]
    fn test_default_rule_allow() {
        let rules = vec![PermissionRule {
            tool_name: "Bash".into(),
            allow: true,
            path_pattern: None,
            command_pattern: None,
        }];
        assert_eq!(
            check_permission(PermissionMode::Default, "Bash", false, &rules),
            PermissionDecision::Allow
        );
    }

    #[test]
    fn test_default_rule_deny() {
        let rules = vec![PermissionRule {
            tool_name: "Bash".into(),
            allow: false,
            path_pattern: None,
            command_pattern: None,
        }];
        assert_eq!(
            check_permission(PermissionMode::Default, "Bash", false, &rules),
            PermissionDecision::Deny
        );
    }

    #[test]
    fn test_default_wildcard_rule() {
        let rules = vec![PermissionRule {
            tool_name: "*".into(),
            allow: true,
            path_pattern: None,
            command_pattern: None,
        }];
        assert_eq!(
            check_permission(PermissionMode::Default, "AnyTool", false, &rules),
            PermissionDecision::Allow
        );
    }

    #[test]
    fn test_permission_mode_default_variant() {
        assert_eq!(PermissionMode::default(), PermissionMode::Default);
    }

    #[test]
    fn test_permission_mode_serde() {
        let json = serde_json::to_string(&PermissionMode::FullAuto).unwrap();
        assert_eq!(json, r#""fullauto""#);
        let parsed: PermissionMode = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, PermissionMode::FullAuto);
    }
}
