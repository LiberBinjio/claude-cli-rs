//! Plugin system — load and manage plugin manifests from disk.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// A plugin manifest (read from `manifest.json` inside each plugin directory).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
    pub entry: String,
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(default)]
    pub commands: Vec<String>,
}

/// Manages a set of loaded plugin manifests.
pub struct PluginManager {
    plugins: Vec<PluginManifest>,
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginManager {
    /// Create an empty plugin manager.
    #[must_use]
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    /// Scan `dir` for subdirectories containing `manifest.json`.
    pub async fn load_from_dir(dir: &Path) -> anyhow::Result<Self> {
        let mut manager = Self::new();
        if !dir.exists() {
            return Ok(manager);
        }
        let mut entries = tokio::fs::read_dir(dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let manifest_path = entry.path().join("manifest.json");
            if manifest_path.exists() {
                if let Ok(content) = tokio::fs::read_to_string(&manifest_path).await {
                    if let Ok(manifest) = serde_json::from_str::<PluginManifest>(&content) {
                        tracing::info!(plugin = %manifest.name, "loaded plugin manifest");
                        manager.plugins.push(manifest);
                    }
                }
            }
        }
        Ok(manager)
    }

    /// Convenience: load from the default plugins directory.
    pub async fn load_plugins() -> anyhow::Result<Self> {
        let dir = claude_utils::platform::data_dir()
            .join("claude-cli-rs")
            .join("plugins");
        Self::load_from_dir(&dir).await
    }

    /// List all loaded manifests.
    #[must_use]
    pub fn list(&self) -> &[PluginManifest] {
        &self.plugins
    }

    /// Find a plugin by name.
    #[must_use]
    pub fn find(&self, name: &str) -> Option<&PluginManifest> {
        self.plugins.iter().find(|p| p.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_manager() {
        let m = PluginManager::new();
        assert!(m.list().is_empty());
        assert!(m.find("x").is_none());
    }

    #[test]
    fn test_manifest_serde() {
        let json = r#"{"name":"test","version":"1.0","entry":"main.js","tools":["a"],"commands":[]}"#;
        let m: PluginManifest = serde_json::from_str(json).unwrap();
        assert_eq!(m.name, "test");
        assert_eq!(m.tools.len(), 1);
    }

    #[tokio::test]
    async fn test_load_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let m = PluginManager::load_from_dir(dir.path()).await.unwrap();
        assert!(m.list().is_empty());
    }

    #[tokio::test]
    async fn test_load_with_plugin() {
        let dir = tempfile::tempdir().unwrap();
        let plugin_dir = dir.path().join("my_plugin");
        std::fs::create_dir_all(&plugin_dir).unwrap();
        std::fs::write(
            plugin_dir.join("manifest.json"),
            r#"{"name":"my_plugin","version":"0.1","entry":"index.js","tools":[],"commands":[]}"#,
        )
        .unwrap();
        let m = PluginManager::load_from_dir(dir.path()).await.unwrap();
        assert_eq!(m.list().len(), 1);
        assert!(m.find("my_plugin").is_some());
    }
}
