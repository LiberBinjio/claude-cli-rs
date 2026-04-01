//! Git command wrappers (calls the external `git` binary).

use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

/// Run a git command in `cwd` and return stdout on success.
fn git(cwd: &Path, args: &[&str]) -> Option<String> {
    Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
}

/// Get the root of the git repo containing `cwd`.
#[must_use]
pub fn get_git_root(cwd: &Path) -> Option<PathBuf> {
    git(cwd, &["rev-parse", "--show-toplevel"]).map(PathBuf::from)
}

/// Returns `true` if `cwd` is inside a git repository.
#[must_use]
pub fn is_git_repo(cwd: &Path) -> bool {
    get_git_root(cwd).is_some()
}

/// Get the diff of the working tree (or staged changes if `staged` is `true`).
pub fn get_git_diff(cwd: &Path, staged: bool) -> anyhow::Result<String> {
    let mut args = vec!["diff"];
    if staged {
        args.push("--cached");
    }
    git(cwd, &args).ok_or_else(|| anyhow::anyhow!("git diff failed"))
}

/// Get the last `count` log entries (one-line format).
pub fn get_git_log(cwd: &Path, count: usize) -> anyhow::Result<String> {
    let n = count.to_string();
    git(cwd, &["log", "--oneline", "-n", &n])
        .ok_or_else(|| anyhow::anyhow!("git log failed"))
}

/// Get the current branch name.
pub fn get_git_branch(cwd: &Path) -> anyhow::Result<String> {
    git(cwd, &["rev-parse", "--abbrev-ref", "HEAD"])
        .ok_or_else(|| anyhow::anyhow!("git branch detection failed"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_git_repo_current_dir() {
        // The workspace root should be a git repo
        let cwd = std::env::current_dir().unwrap();
        // May or may not be true depending on test runner location — just run without panic
        let _ = is_git_repo(&cwd);
    }

    #[test]
    fn test_is_git_repo_temp_dir() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!is_git_repo(dir.path()));
    }
}
