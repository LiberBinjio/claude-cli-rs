//! `GlobTool` — find files matching a glob pattern, respecting `.gitignore`.

use async_trait::async_trait;
use globset::Glob;
use ignore::WalkBuilder;
use serde::Deserialize;
use serde_json::Value;

use claude_core::tool::{PermissionCheck, Tool, ToolInputSchema, ToolResult, ToolUseContext};

/// Maximum number of matches to return.
const MAX_RESULTS: usize = 1000;

/// Searches for files matching a glob pattern, respecting `.gitignore`.
pub struct GlobTool;

#[derive(Debug, Deserialize)]
struct GlobInput {
    pattern: String,
    #[serde(default)]
    path: Option<String>,
}

#[async_trait]
impl Tool for GlobTool {
    #[inline]
    fn name(&self) -> &str {
        "Glob"
    }

    fn description(&self) -> &str {
        include_str!("prompts/glob.txt")
    }

    fn input_schema(&self) -> ToolInputSchema {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": { "type": "string", "description": "Glob pattern (e.g. **/*.rs)" },
                "path": { "type": "string", "description": "Base directory to search in (default: cwd)" }
            },
            "required": ["pattern"]
        })
    }

    #[inline]
    fn is_read_only(&self, _input: &Value) -> bool {
        true
    }

    #[inline]
    fn needs_permission(&self, _input: &Value) -> PermissionCheck {
        PermissionCheck::Allowed
    }

    async fn call(&self, input: Value, ctx: &mut ToolUseContext) -> anyhow::Result<ToolResult> {
        let params: GlobInput = serde_json::from_value(input)?;
        let base = match &params.path {
            Some(p) => {
                let pb = std::path::PathBuf::from(p);
                if pb.is_absolute() {
                    pb
                } else {
                    ctx.cwd.join(pb)
                }
            }
            None => ctx.cwd.clone(),
        };

        let glob = Glob::new(&params.pattern)?.compile_matcher();
        let walker = WalkBuilder::new(&base)
            .hidden(false)
            .git_ignore(true)
            .build();

        let mut matches: Vec<String> = Vec::new();
        for entry in walker.flatten() {
            if matches.len() >= MAX_RESULTS {
                break;
            }
            let path = entry.path();
            let rel = path.strip_prefix(&base).unwrap_or(path);
            if glob.is_match(rel) || glob.is_match(path) {
                matches.push(path.display().to_string());
            }
        }

        if matches.is_empty() {
            return Ok(ToolResult::text("No files matched the pattern."));
        }

        let total = matches.len();
        let mut output = matches.join("\n");
        if total >= MAX_RESULTS {
            output.push_str(&format!("\n... (truncated at {MAX_RESULTS} results)"));
        }
        Ok(ToolResult::text(output))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn ctx(dir: &TempDir) -> ToolUseContext {
        ToolUseContext {
            cwd: dir.path().to_path_buf(),
            permission_mode: claude_core::permission::PermissionMode::Default,
            tool_use_id: "test".into(),
            session_id: "s".into(),
        }
    }

    #[tokio::test]
    async fn matches_files() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("a.rs"), "").unwrap();
        std::fs::write(dir.path().join("b.txt"), "").unwrap();

        let tool = GlobTool;
        let result = tool
            .call(
                serde_json::json!({ "pattern": "*.rs" }),
                &mut ctx(&dir),
            )
            .await
            .unwrap();
        let text = result.content[0].text.as_deref().unwrap();
        assert!(text.contains("a.rs"));
        assert!(!text.contains("b.txt"));
    }

    #[tokio::test]
    async fn no_matches() {
        let dir = TempDir::new().unwrap();
        let tool = GlobTool;
        let result = tool
            .call(
                serde_json::json!({ "pattern": "*.xyz" }),
                &mut ctx(&dir),
            )
            .await
            .unwrap();
        let text = result.content[0].text.as_deref().unwrap();
        assert!(text.contains("No files matched"));
    }

    #[tokio::test]
    async fn recursive_subdirectories() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("sub").join("c.rs"), "").unwrap();

        let tool = GlobTool;
        let result = tool
            .call(
                serde_json::json!({ "pattern": "**/*.rs" }),
                &mut ctx(&dir),
            )
            .await
            .unwrap();
        let text = result.content[0].text.as_deref().unwrap();
        assert!(text.contains("c.rs"));
    }

    #[tokio::test]
    async fn custom_base_path() {
        let dir = TempDir::new().unwrap();
        let sub = dir.path().join("nested");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("x.txt"), "").unwrap();

        let tool = GlobTool;
        let result = tool
            .call(
                serde_json::json!({ "pattern": "*.txt", "path": sub.to_str().unwrap() }),
                &mut ctx(&dir),
            )
            .await
            .unwrap();
        let text = result.content[0].text.as_deref().unwrap();
        assert!(text.contains("x.txt"));
    }

    #[test]
    fn is_read_only_always() {
        let tool = GlobTool;
        assert!(tool.is_read_only(&serde_json::json!({})));
    }

    #[test]
    fn permission_always_allowed() {
        let tool = GlobTool;
        assert_eq!(
            tool.needs_permission(&serde_json::json!({})),
            PermissionCheck::Allowed
        );
    }
}
