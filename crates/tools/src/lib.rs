//! Claude Code — built-in tool implementations.
//!
//! Provides all built-in tools (Bash, File*, Glob, Grep, Web*, Agent, MCP, etc.)
//! organized into priority tiers: P0 (core), P0b (file ops), P1 (network/agent),
//! P2 (tasks/config), P3 (placeholders).

// Tool implementations (dev3-owned unless noted)
pub mod bash;
pub mod grep;
pub mod web_fetch;
pub mod web_search;
pub mod agent;
pub mod mcp_tool;
pub mod todo_write;
pub mod config_tool;
pub mod task_create;
pub mod task_get;
pub mod task_update;
pub mod task_list;
pub mod task_stop;
pub mod task_output;
pub mod lsp;
pub mod notebook_edit;
pub mod skill;
pub mod team_create;
pub mod team_delete;
pub mod send_message;

// Tool implementations (dev4-owned)
pub mod file_read;
pub mod file_write;
pub mod file_edit;
pub mod glob;

// Shared infrastructure
pub mod registry;
pub mod shared;

// Re-exports
pub use registry::ToolRegistry;

use std::sync::Arc;

/// Register P0 tools (Bash, Grep) — core execution.
pub fn register_p0_tools(registry: &mut ToolRegistry) {
    registry.register(Arc::new(bash::BashTool));
    registry.register(Arc::new(grep::GrepTool));
}

/// Register P0b tools (FileRead, FileWrite, FileEdit, Glob) — file operations (dev4).
pub fn register_p0b_tools(registry: &mut ToolRegistry) {
    registry.register(Arc::new(file_read::FileReadTool));
    registry.register(Arc::new(file_write::FileWriteTool));
    registry.register(Arc::new(file_edit::FileEditTool));
    registry.register(Arc::new(glob::GlobTool));
}

/// Register P1 tools (WebFetch, WebSearch, Agent) — network and delegation.
/// Note: `McpProxyTool` is dynamically registered per-server, not here.
pub fn register_p1_tools(registry: &mut ToolRegistry) {
    registry.register(Arc::new(web_fetch::WebFetchTool));
    registry.register(Arc::new(web_search::WebSearchTool));
    registry.register(Arc::new(agent::AgentTool));
}

/// Register P2 tools (TodoWrite, Config, Task*, LSP, NotebookEdit).
pub fn register_p2_tools(registry: &mut ToolRegistry) {
    registry.register(Arc::new(todo_write::TodoWriteTool));
    registry.register(Arc::new(config_tool::ConfigTool));
    registry.register(Arc::new(task_create::TaskCreateTool));
    registry.register(Arc::new(task_get::TaskGetTool));
    registry.register(Arc::new(task_update::TaskUpdateTool));
    registry.register(Arc::new(task_list::TaskListTool));
    registry.register(Arc::new(task_stop::TaskStopTool));
    registry.register(Arc::new(task_output::TaskOutputTool));
    registry.register(Arc::new(lsp::LspTool));
    registry.register(Arc::new(notebook_edit::NotebookEditTool));
}

/// Register P3 tools (Skill, Team*, SendMessage) — placeholder/future.
pub fn register_p3_tools(registry: &mut ToolRegistry) {
    registry.register(Arc::new(skill::SkillTool));
    registry.register(Arc::new(team_create::TeamCreateTool));
    registry.register(Arc::new(team_delete::TeamDeleteTool));
    registry.register(Arc::new(send_message::SendMessageTool));
}

/// Register all built-in tools (P0 through P3) into an existing registry.
pub fn register_all_tools(registry: &mut ToolRegistry) {
    register_p0_tools(registry);
    register_p0b_tools(registry);
    register_p1_tools(registry);
    register_p2_tools(registry);
    register_p3_tools(registry);
}

/// Create a ToolRegistry with all tools pre-registered.
#[must_use]
pub fn create_default_registry() -> ToolRegistry {
    let mut registry = ToolRegistry::new();
    register_all_tools(&mut registry);
    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_default_registry() {
        let registry = create_default_registry();
        assert!(registry.find("Bash").is_some());
        assert!(registry.find("Grep").is_some());
        assert!(registry.find("FileRead").is_some());
        assert!(registry.find("WebFetch").is_some());
        assert!(registry.find("TodoWrite").is_some());
        assert!(registry.find("Skill").is_some());
    }

    #[test]
    fn test_all_tools_registered() {
        let registry = create_default_registry();
        let names = registry.names();
        // 23 total built-in tools (excluding dynamic MCP proxy)
        assert_eq!(names.len(), 23, "Expected 23 tools, got: {names:?}");
    }

    #[test]
    fn test_create_default_registry_has_tools() {
        let registry = create_default_registry();
        assert!(
            registry.all().len() >= 20,
            "expected at least 20 tools, got {}",
            registry.all().len()
        );
    }

    #[test]
    fn test_all_register_functions_exist() {
        let mut r = ToolRegistry::new();
        register_p0_tools(&mut r);
        let p0_count = r.all().len();
        register_p0b_tools(&mut r);
        let p0b_count = r.all().len() - p0_count;
        register_p1_tools(&mut r);
        let p1_count = r.all().len() - p0_count - p0b_count;
        register_p2_tools(&mut r);
        register_p3_tools(&mut r);
        let total = r.all().len();
        assert!(p0_count >= 2, "P0 should have >= 2 tools, got {p0_count}");
        assert!(p0b_count >= 4, "P0b should have >= 4 tools, got {p0b_count}");
        assert!(p1_count >= 3, "P1 should have >= 3 tools, got {p1_count}");
        assert!(total >= 20, "total should be >= 20, got {total}");
    }

    #[test]
    fn test_register_all_tools_matches_create_default() {
        let mut r = ToolRegistry::new();
        register_all_tools(&mut r);
        let default = create_default_registry();
        assert_eq!(r.all().len(), default.all().len());
        assert_eq!(r.names(), default.names());
    }
}
