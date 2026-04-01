//! ConfigTool: read/write ~/.claude/config.json settings.

use claude_core::tool::{PermissionCheck, Tool, ToolInputSchema, ToolResult, ToolUseContext};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

/// Tool for reading and writing configuration values.
pub struct ConfigTool;

#[async_trait]
impl Tool for ConfigTool {
    fn name(&self) -> &str { "Config" }

    fn description(&self) -> &str {
        "Read or write configuration values in ~/.claude/config.json."
    }

    fn input_schema(&self) -> ToolInputSchema {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["get", "set", "list"],
                    "description": "Action: get a value, set a value, or list all"
                },
                "key": {
                    "type": "string",
                    "description": "Config key (for get/set)"
                },
                "value": {
                    "description": "Config value (for set)"
                }
            },
            "required": ["action"]
        })
    }

    fn is_read_only(&self, input: &Value) -> bool {
        input
            .get("action")
            .and_then(|v| v.as_str())
            .is_some_and(|a| a == "get" || a == "list")
    }

    fn needs_permission(&self, input: &Value) -> PermissionCheck {
        if self.is_read_only(input) {
            PermissionCheck::Allowed
        } else {
            PermissionCheck::NeedsAsk
        }
    }

    async fn call(&self, input: Value, _ctx: &mut ToolUseContext) -> anyhow::Result<ToolResult> {
        let action = input
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing 'action' parameter"))?;

        let Some(dir) = crate::shared::claude_home_dir() else {
            return Ok(ToolResult::error("Cannot determine home directory"));
        };
        let config_path = dir.join("config.json");

        match action {
            "get" => {
                let key = input
                    .get("key")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("'get' requires 'key'"))?;
                let config = load_config(&config_path);
                match config.get(key) {
                    Some(val) => Ok(ToolResult::text(format!("{key} = {val}"))),
                    None => Ok(ToolResult::text(format!("{key}: not set"))),
                }
            }
            "set" => {
                let key = input
                    .get("key")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("'set' requires 'key'"))?;
                let value = input
                    .get("value")
                    .ok_or_else(|| anyhow::anyhow!("'set' requires 'value'"))?;
                let mut config = load_config(&config_path);
                config.insert(key.to_string(), value.clone());
                save_config(&config_path, &config)?;
                Ok(ToolResult::text(format!("Set {key} = {value}")))
            }
            "list" => {
                let config = load_config(&config_path);
                if config.is_empty() {
                    Ok(ToolResult::text("No configuration values set."))
                } else {
                    let text = config
                        .iter()
                        .map(|(k, v)| format!("  {k} = {v}"))
                        .collect::<Vec<_>>()
                        .join("\n");
                    Ok(ToolResult::text(format!("Configuration:\n{text}")))
                }
            }
            other => Ok(ToolResult::error(format!("Unknown action: {other}"))),
        }
    }
}

fn load_config(path: &std::path::Path) -> HashMap<String, Value> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_config(path: &std::path::Path, config: &HashMap<String, Value>) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(config)?;
    std::fs::write(path, json)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema() {
        let tool = ConfigTool;
        assert_eq!(tool.name(), "Config");
        let schema = tool.input_schema();
        assert_eq!(schema["required"][0], "action");
    }

    #[test]
    fn test_is_read_only_dynamic() {
        let tool = ConfigTool;
        assert!(tool.is_read_only(&serde_json::json!({"action": "get"})));
        assert!(tool.is_read_only(&serde_json::json!({"action": "list"})));
        assert!(!tool.is_read_only(&serde_json::json!({"action": "set"})));
    }
}
