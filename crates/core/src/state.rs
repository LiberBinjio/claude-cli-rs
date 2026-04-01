//! Application state shared across the system.

use crate::config::AppConfig;
use crate::message::Message;
use crate::permission::PermissionMode;
use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Shared application state, safe for concurrent access.
pub struct AppState {
    /// Conversation message history.
    pub messages: Arc<RwLock<Vec<Message>>>,
    /// Application configuration.
    pub config: Arc<RwLock<AppConfig>>,
    /// Unique session identifier.
    pub session_id: String,
    /// Current working directory.
    pub cwd: PathBuf,
    /// Active permission mode.
    pub permission_mode: PermissionMode,
    /// Total cost in micro-dollars (USD * 1_000_000).
    pub total_cost_usd: Arc<AtomicU64>,
    /// Total input tokens consumed.
    pub total_input_tokens: Arc<AtomicU64>,
    /// Total output tokens generated.
    pub total_output_tokens: Arc<AtomicU64>,
}

impl AppState {
    /// Create a new `AppState` with a working directory and configuration.
    #[must_use]
    pub fn new(cwd: PathBuf, config: AppConfig) -> Self {
        Self {
            messages: Arc::new(RwLock::new(Vec::new())),
            permission_mode: config.permission_mode,
            config: Arc::new(RwLock::new(config)),
            session_id: uuid::Uuid::new_v4().to_string(),
            cwd,
            total_cost_usd: Arc::new(AtomicU64::new(0)),
            total_input_tokens: Arc::new(AtomicU64::new(0)),
            total_output_tokens: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Get the total cost in USD as a floating-point value.
    #[must_use]
    #[inline]
    pub fn cost_usd(&self) -> f64 {
        let micros = self
            .total_cost_usd
            .load(std::sync::atomic::Ordering::Relaxed);
        micros as f64 / 1_000_000.0
    }

    /// Add to the total cost (in micro-dollars).
    #[inline]
    pub fn add_cost(&self, micro_usd: u64) {
        self.total_cost_usd
            .fetch_add(micro_usd, std::sync::atomic::Ordering::Relaxed);
    }

    /// Get total input tokens.
    #[must_use]
    #[inline]
    pub fn input_tokens(&self) -> u64 {
        self.total_input_tokens
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Get total output tokens.
    #[must_use]
    #[inline]
    pub fn output_tokens(&self) -> u64 {
        self.total_output_tokens
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Record token usage.
    #[inline]
    pub fn add_tokens(&self, input: u64, output: u64) {
        self.total_input_tokens
            .fetch_add(input, std::sync::atomic::Ordering::Relaxed);
        self.total_output_tokens
            .fetch_add(output, std::sync::atomic::Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_state() {
        let state = AppState::new(PathBuf::from("/tmp"), AppConfig::default());
        assert_eq!(state.cwd, PathBuf::from("/tmp"));
        assert_eq!(state.permission_mode, PermissionMode::Default);
        assert!(!state.session_id.is_empty());
        assert_eq!(state.cost_usd(), 0.0);
        assert_eq!(state.input_tokens(), 0);
        assert_eq!(state.output_tokens(), 0);
    }

    #[test]
    fn test_add_cost() {
        let state = AppState::new(PathBuf::from("."), AppConfig::default());
        state.add_cost(1_500_000); // $1.50
        assert!((state.cost_usd() - 1.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_add_tokens() {
        let state = AppState::new(PathBuf::from("."), AppConfig::default());
        state.add_tokens(100, 200);
        state.add_tokens(50, 30);
        assert_eq!(state.input_tokens(), 150);
        assert_eq!(state.output_tokens(), 230);
    }
}
