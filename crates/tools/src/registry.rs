//! Tool registry: manages all available tools by name.

use claude_core::tool::Tool;
use std::collections::HashMap;
use std::sync::Arc;

/// Registry holding all available tools, preserving insertion order.
pub struct ToolRegistry {
    tools: Vec<Arc<dyn Tool>>,
    index: HashMap<String, usize>,
}

impl ToolRegistry {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            tools: Vec::new(),
            index: HashMap::new(),
        }
    }

    /// Register a tool. Overwrites any existing tool with the same name.
    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        let name = tool.name().to_string();
        if let Some(&idx) = self.index.get(&name) {
            self.tools[idx] = tool;
        } else {
            let idx = self.tools.len();
            self.index.insert(name, idx);
            self.tools.push(tool);
        }
    }

    /// Find a tool by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.index.get(name).map(|&idx| Arc::clone(&self.tools[idx]))
    }

    /// Alias for `get` (backward compatibility).
    #[must_use]
    #[inline]
    pub fn find(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.get(name)
    }

    /// Return all registered tools in insertion order.
    #[must_use]
    #[inline]
    pub fn all(&self) -> &[Arc<dyn Tool>] {
        &self.tools
    }

    /// Return names of all registered tools.
    #[must_use]
    pub fn names(&self) -> Vec<String> {
        self.tools.iter().map(|t| t.name().to_string()).collect()
    }

    /// Generate API-compatible JSON Schema definitions for all tools.
    #[must_use]
    pub fn to_api_schemas(&self) -> Vec<serde_json::Value> {
        self.tools
            .iter()
            .map(|t| {
                serde_json::json!({
                    "name": t.name(),
                    "description": t.description(),
                    "input_schema": t.input_schema(),
                })
            })
            .collect()
    }

    /// Alias for `to_api_schemas` (backward compatibility).
    #[must_use]
    #[inline]
    pub fn all_definitions(&self) -> Vec<serde_json::Value> {
        self.to_api_schemas()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claude_core::tool::{PermissionCheck, Tool, ToolInputSchema, ToolResult, ToolUseContext};
    use serde_json::Value;

    struct DummyTool(&'static str);

    #[async_trait::async_trait]
    impl Tool for DummyTool {
        fn name(&self) -> &str { self.0 }
        fn description(&self) -> &str { "dummy" }
        fn input_schema(&self) -> Value {
            serde_json::json!({"type": "object", "properties": {}})
        }
        fn is_read_only(&self, _: &Value) -> bool { true }
        async fn call(&self, _: Value, _: &mut ToolUseContext) -> anyhow::Result<ToolResult> {
            Ok(ToolResult::text("ok"))
        }
    }

    #[test]
    fn test_register_and_find() {
        let mut reg = ToolRegistry::new();
        reg.register(Arc::new(DummyTool("alpha")));
        reg.register(Arc::new(DummyTool("beta")));
        assert!(reg.find("alpha").is_some());
        assert!(reg.find("beta").is_some());
        assert!(reg.find("gamma").is_none());
    }

    #[test]
    fn test_names() {
        let mut reg = ToolRegistry::new();
        reg.register(Arc::new(DummyTool("a")));
        reg.register(Arc::new(DummyTool("b")));
        assert_eq!(reg.names(), vec!["a", "b"]);
    }

    #[test]
    fn test_overwrite() {
        let mut reg = ToolRegistry::new();
        reg.register(Arc::new(DummyTool("x")));
        reg.register(Arc::new(DummyTool("x")));
        assert_eq!(reg.all().len(), 1);
    }

    #[test]
    fn test_api_schemas() {
        let mut reg = ToolRegistry::new();
        reg.register(Arc::new(DummyTool("tool1")));
        let schemas = reg.to_api_schemas();
        assert_eq!(schemas.len(), 1);
        assert_eq!(schemas[0]["name"], "tool1");
        assert_eq!(schemas[0]["description"], "dummy");
    }
}
