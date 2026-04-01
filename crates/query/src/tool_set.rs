//! Lightweight tool registry for the query engine.
//!
//! Wraps a collection of [`Tool`] implementations so the query loop
//! can discover and invoke tools without depending on the full
//! `claude_tools` crate (which may not be built yet).

use claude_core::tool::Tool;
use std::collections::HashMap;
use std::sync::Arc;

/// Minimal tool registry: name → implementation.
#[derive(Default, Clone)]
pub struct ToolSet {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolSet {
    /// Create an empty `ToolSet`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a tool.
    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    /// Look up a tool by name.
    #[must_use]
    pub fn find(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    /// All registered tool names.
    #[must_use]
    pub fn names(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    /// Generate API-compatible tool schemas for all registered tools.
    #[must_use]
    pub fn to_api_schemas(&self) -> Vec<serde_json::Value> {
        self.tools
            .values()
            .map(|t| {
                serde_json::json!({
                    "name": t.name(),
                    "description": t.description(),
                    "input_schema": t.input_schema(),
                })
            })
            .collect()
    }

    /// Number of registered tools.
    #[must_use]
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Whether there are no tools registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_tool_set() {
        let ts = ToolSet::new();
        assert!(ts.is_empty());
        assert_eq!(ts.len(), 0);
        assert!(ts.find("no_such_tool").is_none());
    }

    #[test]
    fn test_names_and_schemas() {
        let ts = ToolSet::new();
        assert!(ts.names().is_empty());
        assert!(ts.to_api_schemas().is_empty());
    }
}
