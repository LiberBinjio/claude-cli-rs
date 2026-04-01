//! Diff generation and edit-apply logic (core of `FileEditTool`).

use thiserror::Error;

/// Error when applying an edit to a document.
#[derive(Debug, Error)]
pub enum EditError {
    /// The search text was not found in the document.
    #[error("old_string not found in content")]
    NotFound,
    /// The search text matches more than once.
    #[error("old_string is ambiguous: found {0} occurrences")]
    Ambiguous(usize),
}

/// Generate a unified diff between `old` and `new` text for `filename`.
#[must_use]
pub fn unified_diff(old: &str, new: &str, filename: &str) -> String {
    use similar::TextDiff;

    let diff = TextDiff::from_lines(old, new);
    let mut out = String::new();
    out.push_str(&format!("--- a/{filename}\n"));
    out.push_str(&format!("+++ b/{filename}\n"));

    for hunk in diff.unified_diff().context_radius(3).iter_hunks() {
        out.push_str(&format!("{hunk}"));
    }
    out
}

/// Replace `old_string` with `new_string` inside `content`.
///
/// `old_string` must appear exactly once; otherwise returns `EditError`.
pub fn apply_edit(content: &str, old_string: &str, new_string: &str) -> Result<String, EditError> {
    if old_string.is_empty() {
        return Err(EditError::NotFound);
    }

    // Use two sequential find() calls for efficiency (CONTRACT optimization).
    let first = match content.find(old_string) {
        Some(pos) => pos,
        None => return Err(EditError::NotFound),
    };

    // Check for a second occurrence starting after the first match.
    let after_first = first + old_string.len();
    if content[after_first..].contains(old_string) {
        // Count total occurrences for the error message.
        let count = content.matches(old_string).count();
        return Err(EditError::Ambiguous(count));
    }

    let mut result = String::with_capacity(content.len() + new_string.len());
    result.push_str(&content[..first]);
    result.push_str(new_string);
    result.push_str(&content[after_first..]);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_edit_exact() {
        let content = "fn hello() {\n    println!(\"hi\");\n}\n";
        let result = apply_edit(content, "println!(\"hi\")", "println!(\"bye\")").unwrap();
        assert!(result.contains("bye"));
        assert!(!result.contains("hi"));
    }

    #[test]
    fn test_apply_edit_not_found() {
        let result = apply_edit("abc", "xyz", "123");
        assert!(matches!(result, Err(EditError::NotFound)));
    }

    #[test]
    fn test_apply_edit_ambiguous() {
        let result = apply_edit("aaa bbb aaa", "aaa", "ccc");
        assert!(matches!(result, Err(EditError::Ambiguous(2))));
    }

    #[test]
    fn test_apply_edit_empty_old_string() {
        let result = apply_edit("hello", "", "world");
        assert!(matches!(result, Err(EditError::NotFound)));
    }

    #[test]
    fn test_apply_edit_multiline() {
        let content = "line1\nline2\nline3\n";
        let result = apply_edit(content, "line2\nline3", "lineA\nlineB").unwrap();
        assert_eq!(result, "line1\nlineA\nlineB\n");
    }

    #[test]
    fn test_unified_diff() {
        let old = "line1\nline2\nline3\n";
        let new = "line1\nlineX\nline3\n";
        let diff = unified_diff(old, new, "test.rs");
        assert!(diff.contains("--- a/test.rs"));
        assert!(diff.contains("+++ b/test.rs"));
        assert!(diff.contains("-line2"));
        assert!(diff.contains("+lineX"));
    }

    #[test]
    fn test_unified_diff_has_hunk_header() {
        let old = "a\nb\nc\n";
        let new = "a\nB\nc\n";
        let diff = unified_diff(old, new, "f.rs");
        assert!(diff.contains("@@"), "unified diff must contain @@ hunk headers");
    }

    #[test]
    fn test_apply_edit_ambiguous_count_is_exact() {
        let content = "foo bar foo baz foo";
        match apply_edit(content, "foo", "qux") {
            Err(EditError::Ambiguous(n)) => assert_eq!(n, 3),
            other => panic!("expected Ambiguous(3), got {:?}", other),
        }
    }

    #[test]
    fn test_apply_edit_replaces_with_empty() {
        let result = apply_edit("hello world", "world", "").unwrap();
        assert_eq!(result, "hello ");
    }

    #[test]
    fn test_apply_edit_preserves_surrounding() {
        let content = "AAA\nBBB\nCCC\n";
        let result = apply_edit(content, "BBB", "XXX").unwrap();
        assert_eq!(result, "AAA\nXXX\nCCC\n");
    }
}
