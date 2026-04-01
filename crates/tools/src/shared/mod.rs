//! Shared state: task manager, home directory resolution.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

/// Global task manager for background tasks.
pub static TASK_MANAGER: OnceLock<Mutex<HashMap<String, TaskEntry>>> = OnceLock::new();

/// Get the task manager, initializing on first access.
#[inline]
pub fn task_manager() -> &'static Mutex<HashMap<String, TaskEntry>> {
    TASK_MANAGER.get_or_init(|| Mutex::new(HashMap::new()))
}

/// A tracked background task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskEntry {
    /// Unique task ID.
    pub id: String,
    /// Human-readable description.
    pub description: String,
    /// Current status.
    pub status: TaskStatus,
    /// Accumulated output text.
    pub output: String,
    /// ISO 8601 creation timestamp.
    pub created_at: String,
}

/// Task lifecycle states.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    /// Waiting to run.
    Pending,
    /// Currently executing.
    Running,
    /// Finished successfully.
    Completed,
    /// Finished with error.
    Failed,
    /// User-cancelled.
    Cancelled,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Running => write!(f, "running"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Create a new task entry and insert it into the global manager.
#[must_use]
pub fn create_task(description: &str) -> String {
    let id = uuid::Uuid::new_v4().to_string();
    let entry = TaskEntry {
        id: id.clone(),
        description: description.to_string(),
        status: TaskStatus::Pending,
        output: String::new(),
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    if let Ok(mut mgr) = task_manager().lock() {
        mgr.insert(id.clone(), entry);
    }
    id
}

/// Resolve the Claude home directory (`~/.claude/`).
#[must_use]
pub fn claude_home_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".claude"))
}
