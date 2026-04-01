//! SkillTool: invoke a predefined skill (placeholder).

use claude_core::tool::{PermissionCheck, Tool, ToolInputSchema, ToolResult, ToolUseContext};
use async_trait::async_trait;
use serde_json::Value;

/// Tool for invoking the skill system.
pub struct SkillTool;

#[async_trait]
impl Tool for SkillTool {
    fn name(&self) -> &str { "Skill" }
    fn description(&self) -> &str { "Invoke a predefined skill from the skill library." }

    fn input_schema(&self) -> ToolInputSchema {
        serde_json::json!({
            "type": "object",
            "properties": {
                "skill_name": { "type": "string", "description": "Name of the skill to invoke" },
                "args": { "type": "object", "description": "Arguments for the skill" }
            },
            "required": ["skill_name"]
        })
    }

    fn is_read_only(&self, _input: &Value) -> bool { true }

    fn needs_permission(&self, _input: &Value) -> PermissionCheck {
        PermissionCheck::Allowed
    }

    async fn call(&self, _input: Value, _ctx: &mut ToolUseContext) -> anyhow::Result<ToolResult> {
        // TODO: Load SKILL.md files and execute skill logic
        Ok(ToolResult::error("Not yet implemented: Skill"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_placeholder() {
        let tool = SkillTool;
        let mut ctx = ToolUseContext {
            cwd: PathBuf::from("."),
            permission_mode: claude_core::permission::PermissionMode::Default,
            tool_use_id: "t".into(),
            session_id: "s".into(),
        };
        let result = tool.call(serde_json::json!({"skill_name": "test"}), &mut ctx).await.unwrap();
        assert!(result.is_error);
    }
}
