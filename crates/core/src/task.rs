//! Background task tracking.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Status of a background task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// Task is queued but not yet running.
    Pending,
    /// Task is currently executing.
    Running,
    /// Task finished successfully.
    Completed,
    /// Task ended with an error.
    Failed,
    /// Task was cancelled by the user.
    Cancelled,
}

/// A tracked background task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique task identifier.
    pub id: String,
    /// Human-readable description.
    pub description: String,
    /// Current status.
    pub status: TaskStatus,
    /// When the task was created.
    pub created_at: DateTime<Utc>,
    /// When the task completed (if it has).
    pub completed_at: Option<DateTime<Utc>>,
}

impl Task {
    /// Create a new pending task.
    #[must_use]
    pub fn new(id: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            description: description.into(),
            status: TaskStatus::Pending,
            created_at: Utc::now(),
            completed_at: None,
        }
    }

    /// Mark the task as running.
    pub fn start(&mut self) {
        self.status = TaskStatus::Running;
    }

    /// Mark the task as completed.
    pub fn complete(&mut self) {
        self.status = TaskStatus::Completed;
        self.completed_at = Some(Utc::now());
    }

    /// Mark the task as failed.
    pub fn fail(&mut self) {
        self.status = TaskStatus::Failed;
        self.completed_at = Some(Utc::now());
    }

    /// Mark the task as cancelled.
    pub fn cancel(&mut self) {
        self.status = TaskStatus::Cancelled;
        self.completed_at = Some(Utc::now());
    }

    /// Whether the task is still active (pending or running).
    #[must_use]
    #[inline]
    pub fn is_active(&self) -> bool {
        matches!(self.status, TaskStatus::Pending | TaskStatus::Running)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_task() {
        let task = Task::new("t1", "do something");
        assert_eq!(task.id, "t1");
        assert_eq!(task.description, "do something");
        assert_eq!(task.status, TaskStatus::Pending);
        assert!(task.completed_at.is_none());
        assert!(task.is_active());
    }

    #[test]
    fn test_task_lifecycle() {
        let mut task = Task::new("t1", "test");
        assert!(task.is_active());

        task.start();
        assert_eq!(task.status, TaskStatus::Running);
        assert!(task.is_active());

        task.complete();
        assert_eq!(task.status, TaskStatus::Completed);
        assert!(!task.is_active());
        assert!(task.completed_at.is_some());
    }

    #[test]
    fn test_task_fail() {
        let mut task = Task::new("t1", "test");
        task.start();
        task.fail();
        assert_eq!(task.status, TaskStatus::Failed);
        assert!(!task.is_active());
    }

    #[test]
    fn test_task_cancel() {
        let mut task = Task::new("t1", "test");
        task.cancel();
        assert_eq!(task.status, TaskStatus::Cancelled);
        assert!(!task.is_active());
    }

    #[test]
    fn test_task_status_serde() {
        let json = serde_json::to_string(&TaskStatus::Running).unwrap();
        assert_eq!(json, r#""running""#);
        let parsed: TaskStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, TaskStatus::Running);
    }
}
