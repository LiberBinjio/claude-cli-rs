//! Local analytics — track events without sending anywhere.

use serde::Serialize;
use tracing::debug;

/// A single analytics event.
#[derive(Debug, Clone, Serialize)]
pub struct AnalyticsEvent {
    pub event_type: String,
    pub properties: serde_json::Value,
    pub timestamp: f64,
}

/// In-process analytics collector (no remote reporting).
pub struct Analytics {
    enabled: bool,
    events: Vec<AnalyticsEvent>,
}

impl Default for Analytics {
    fn default() -> Self {
        Self::new()
    }
}

impl Analytics {
    /// Create a new enabled analytics collector.
    #[must_use]
    pub fn new() -> Self {
        Self {
            enabled: true,
            events: Vec::new(),
        }
    }

    /// Disable event collection.
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Record an event.
    pub fn track(&mut self, event_type: &str, properties: serde_json::Value) {
        if !self.enabled {
            return;
        }
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();
        let event = AnalyticsEvent {
            event_type: event_type.to_owned(),
            properties,
            timestamp: ts,
        };
        debug!(event = event.event_type, "analytics event tracked");
        self.events.push(event);
    }

    /// Record a query-start event with model info.
    pub fn track_query_start(&mut self, model: &str) {
        self.track("query_start", serde_json::json!({ "model": model }));
    }

    /// Record a tool-use event.
    pub fn track_tool_use(&mut self, tool_name: &str) {
        self.track("tool_use", serde_json::json!({ "tool": tool_name }));
    }

    /// All collected events.
    #[must_use]
    pub fn events(&self) -> &[AnalyticsEvent] {
        &self.events
    }

    /// Number of events collected.
    #[must_use]
    pub fn event_count(&self) -> usize {
        self.events.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_track_event() {
        let mut a = Analytics::new();
        a.track("test", serde_json::json!({"k": "v"}));
        assert_eq!(a.event_count(), 1);
        assert_eq!(a.events()[0].event_type, "test");
    }

    #[test]
    fn test_disabled_analytics() {
        let mut a = Analytics::new();
        a.disable();
        a.track("test", serde_json::json!({}));
        assert_eq!(a.event_count(), 0);
    }

    #[test]
    fn test_track_query_start() {
        let mut a = Analytics::new();
        a.track_query_start("claude-sonnet");
        assert_eq!(a.event_count(), 1);
    }

    #[test]
    fn test_track_tool_use() {
        let mut a = Analytics::new();
        a.track_tool_use("BashTool");
        assert_eq!(a.events()[0].event_type, "tool_use");
    }
}
