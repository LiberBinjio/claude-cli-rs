//! Claude Code — QueryEngine: conversation orchestration

pub mod compact;
pub mod engine;
pub mod query_loop;
pub mod system_prompt;
pub mod tool_set;

pub use engine::{QueryEngine, QueryEvent};
pub use tool_set::ToolSet;
