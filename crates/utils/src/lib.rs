//! Claude Code — shared utilities (git, shell, fs, diff, tokens, markdown, platform, env).

pub mod platform;
pub mod env;
pub mod fs;
pub mod git;
pub mod shell;
pub mod diff;
pub mod tokens;
pub mod markdown;

pub use diff::{apply_edit, unified_diff, EditError};
pub use tokens::{estimate_token_count, truncate_to_token_budget};
