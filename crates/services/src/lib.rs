//! Claude Code — services (session, analytics, compact, plugins, tips, cost).

pub mod session;
pub mod analytics;
pub mod compact;
pub mod plugins;
pub mod tips;
pub mod cost;

pub use analytics::Analytics;
pub use compact::CompactService;
pub use cost::CostTracker;
pub use plugins::PluginManager;
pub use session::SessionMetadata;
