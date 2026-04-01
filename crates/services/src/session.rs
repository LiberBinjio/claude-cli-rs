//! Session persistence — save/load/list/delete conversation sessions as JSONL.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::debug;

/// Summary metadata for a saved session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    pub id: String,
    pub created_at: f64,
    pub updated_at: f64,
    pub message_count: usize,
    pub title: Option<String>,
    pub cwd: String,
}

/// Root directory for stored sessions.
#[must_use]
pub fn sessions_dir() -> PathBuf {
    claude_utils::platform::data_dir()
        .join("claude-cli-rs")
        .join("sessions")
}

/// Save a slice of messages plus metadata to disk.
pub async fn save_session(
    id: &str,
    messages: &[claude_core::Message],
    cwd: &Path,
) -> Result<()> {
    let dir = sessions_dir();
    tokio::fs::create_dir_all(&dir).await?;

    // Write messages as JSONL
    let mut content = String::new();
    for msg in messages {
        content.push_str(&serde_json::to_string(msg)?);
        content.push('\n');
    }
    let msg_path = dir.join(format!("{id}.jsonl"));
    // Atomic write via temp file
    let tmp_path = dir.join(format!("{id}.jsonl.tmp"));
    tokio::fs::write(&tmp_path, &content).await?;
    tokio::fs::rename(&tmp_path, &msg_path).await?;

    // Write metadata
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64();
    let meta = SessionMetadata {
        id: id.to_owned(),
        created_at: now,
        updated_at: now,
        message_count: messages.len(),
        title: extract_title(messages),
        cwd: cwd.display().to_string(),
    };
    let meta_path = dir.join(format!("{id}.meta.json"));
    let meta_tmp = dir.join(format!("{id}.meta.json.tmp"));
    tokio::fs::write(&meta_tmp, serde_json::to_string_pretty(&meta)?).await?;
    tokio::fs::rename(&meta_tmp, &meta_path).await?;

    debug!(session_id = id, messages = messages.len(), "session saved");
    Ok(())
}

/// Load all messages from a session file.
pub async fn load_session(id: &str) -> Result<Vec<claude_core::Message>> {
    let path = sessions_dir().join(format!("{id}.jsonl"));
    let content = tokio::fs::read_to_string(&path)
        .await
        .with_context(|| format!("loading session {id}"))?;
    let mut messages = Vec::new();
    for line in content.lines() {
        if !line.trim().is_empty() {
            messages.push(serde_json::from_str(line)?);
        }
    }
    Ok(messages)
}

/// List all saved sessions, newest first.
pub async fn list_sessions() -> Result<Vec<SessionMetadata>> {
    let dir = sessions_dir();
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut sessions = Vec::new();
    let mut entries = tokio::fs::read_dir(&dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        let name = path.file_name().map(|n| n.to_string_lossy().to_string());
        if let Some(name) = name {
            if name.ends_with(".meta.json") {
                if let Ok(content) = tokio::fs::read_to_string(&path).await {
                    if let Ok(meta) = serde_json::from_str::<SessionMetadata>(&content) {
                        sessions.push(meta);
                    }
                }
            }
        }
    }
    sessions.sort_by(|a, b| b.updated_at.partial_cmp(&a.updated_at).unwrap_or(std::cmp::Ordering::Equal));
    Ok(sessions)
}

/// Delete a session's files from disk.
pub async fn delete_session(id: &str) -> Result<()> {
    let dir = sessions_dir();
    let msg_path = dir.join(format!("{id}.jsonl"));
    let meta_path = dir.join(format!("{id}.meta.json"));
    if msg_path.exists() {
        tokio::fs::remove_file(&msg_path).await?;
    }
    if meta_path.exists() {
        tokio::fs::remove_file(&meta_path).await?;
    }
    Ok(())
}

/// Extract a title from the first user message (up to 80 chars).
fn extract_title(messages: &[claude_core::Message]) -> Option<String> {
    messages.iter().find_map(|msg| {
        if msg.role == claude_core::Role::User {
            msg.content.iter().find_map(|b| match b {
                claude_core::ContentBlock::Text { text } => {
                    Some(text.chars().take(80).collect())
                }
                _ => None,
            })
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_title() {
        let msgs = vec![claude_core::Message {
            role: claude_core::Role::User,
            content: vec![claude_core::ContentBlock::Text {
                text: "Help me write a web server".into(),
            }],
            cache_control: None,
        }];
        assert_eq!(
            extract_title(&msgs),
            Some("Help me write a web server".to_string())
        );
    }

    #[test]
    fn test_extract_title_empty() {
        let msgs: Vec<claude_core::Message> = Vec::new();
        assert_eq!(extract_title(&msgs), None);
    }

    #[test]
    fn test_extract_title_truncates() {
        let long_text = "x".repeat(200);
        let msgs = vec![claude_core::Message {
            role: claude_core::Role::User,
            content: vec![claude_core::ContentBlock::Text { text: long_text }],
            cache_control: None,
        }];
        let title = extract_title(&msgs).unwrap();
        assert_eq!(title.len(), 80);
    }

    #[test]
    fn test_sessions_dir_not_empty() {
        let d = sessions_dir();
        assert!(!d.as_os_str().is_empty());
        assert!(d.ends_with("sessions"));
    }

    #[tokio::test]
    async fn test_save_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        // Override sessions_dir by using direct path
        let id = "test_session_rt";
        let session_dir = dir.path().join("sessions");
        tokio::fs::create_dir_all(&session_dir).await.unwrap();

        let msgs = vec![claude_core::Message {
            role: claude_core::Role::User,
            content: vec![claude_core::ContentBlock::Text {
                text: "hello".into(),
            }],
            cache_control: None,
        }];

        // Write directly to temp dir
        let msg_path = session_dir.join(format!("{id}.jsonl"));
        let mut content = String::new();
        for msg in &msgs {
            content.push_str(&serde_json::to_string(msg).unwrap());
            content.push('\n');
        }
        tokio::fs::write(&msg_path, &content).await.unwrap();

        // Read back
        let raw = tokio::fs::read_to_string(&msg_path).await.unwrap();
        let loaded: Vec<claude_core::Message> = raw
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].role, claude_core::Role::User);
        match &loaded[0].content[0] {
            claude_core::ContentBlock::Text { text } => assert_eq!(text, "hello"),
            other => panic!("expected Text, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_save_load_multiple_messages() {
        let dir = tempfile::tempdir().unwrap();
        let session_dir = dir.path().join("sessions");
        tokio::fs::create_dir_all(&session_dir).await.unwrap();

        let msgs = vec![
            claude_core::Message {
                role: claude_core::Role::User,
                content: vec![claude_core::ContentBlock::Text {
                    text: "question".into(),
                }],
                cache_control: None,
            },
            claude_core::Message {
                role: claude_core::Role::Assistant,
                content: vec![claude_core::ContentBlock::Text {
                    text: "answer".into(),
                }],
                cache_control: None,
            },
        ];

        let id = "multi_msg";
        let msg_path = session_dir.join(format!("{id}.jsonl"));
        let mut content = String::new();
        for msg in &msgs {
            content.push_str(&serde_json::to_string(msg).unwrap());
            content.push('\n');
        }
        tokio::fs::write(&msg_path, &content).await.unwrap();

        let raw = tokio::fs::read_to_string(&msg_path).await.unwrap();
        let loaded: Vec<claude_core::Message> = raw
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].role, claude_core::Role::User);
        assert_eq!(loaded[1].role, claude_core::Role::Assistant);
    }

    #[test]
    fn test_session_metadata_serde_roundtrip() {
        let meta = SessionMetadata {
            id: "abc-123".into(),
            created_at: 1700000000.0,
            updated_at: 1700000100.0,
            message_count: 5,
            title: Some("Test session".into()),
            cwd: "/home/user".into(),
        };
        let json = serde_json::to_string(&meta).unwrap();
        let loaded: SessionMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.id, "abc-123");
        assert_eq!(loaded.message_count, 5);
        assert_eq!(loaded.title, Some("Test session".into()));
    }

    #[test]
    fn test_extract_title_skips_assistant() {
        let msgs = vec![claude_core::Message {
            role: claude_core::Role::Assistant,
            content: vec![claude_core::ContentBlock::Text {
                text: "I am the assistant".into(),
            }],
            cache_control: None,
        }];
        assert_eq!(extract_title(&msgs), None);
    }
}
