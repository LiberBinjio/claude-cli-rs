//! Bottom status bar — model, token usage, cost, permission mode.

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::theme::Theme;

/// Data shown in the status bar.
#[derive(Debug, Clone)]
pub struct StatusInfo {
    /// Model name.
    pub model: String,
    /// Cumulative input + output tokens.
    pub total_tokens: u64,
    /// Cumulative cost in USD.
    pub total_cost_usd: f64,
    /// Session identifier.
    pub session_id: String,
    /// Active permission mode label.
    pub permission_mode: String,
    /// Whether a valid API key is available.
    pub auth_ok: bool,
    /// Whether the system is ready (not loading).
    pub is_ready: bool,
}

impl Default for StatusInfo {
    fn default() -> Self {
        Self {
            model: "claude-sonnet-4-20250514".into(),
            total_tokens: 0,
            total_cost_usd: 0.0,
            session_id: String::new(),
            permission_mode: "default".into(),
            auth_ok: true,
            is_ready: true,
        }
    }
}

/// Render the status bar into a single-line area.
pub fn render_status_line(frame: &mut Frame, area: Rect, info: &StatusInfo, theme: &Theme) {
    let cost_str = if info.total_cost_usd > 0.0 {
        format!("${:.4}", info.total_cost_usd)
    } else {
        "$0.00".into()
    };

    let tokens_str = format_tokens(info.total_tokens);

    // Left: mode indicator
    let mode_span = if !info.auth_ok {
        Span::styled(
            " \u{26a0} No API key \u{2014} Set ANTHROPIC_API_KEY ",
            Style::default()
                .fg(theme.error)
                .add_modifier(Modifier::BOLD),
        )
    } else if info.is_ready {
        Span::styled(" Ready ", Style::default().fg(theme.success))
    } else {
        Span::styled(" Thinking... ", Style::default().fg(theme.spinner))
    };

    let line = Line::from(vec![
        mode_span,
        Span::raw("\u{2502} "),
        Span::styled(
            " \u{25c6} ",
            Style::default().fg(theme.primary),
        ),
        Span::styled(info.model.clone(), Style::default().fg(theme.fg)),
        Span::raw(" \u{2502} "),
        Span::styled(tokens_str, Style::default().fg(theme.info)),
        Span::raw(" \u{2502} "),
        Span::styled(cost_str, Style::default().fg(theme.success)),
        Span::raw(" \u{2502} mode: "),
        Span::styled(
            info.permission_mode.clone(),
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    let widget = Paragraph::new(line).style(Style::default().bg(theme.bg).fg(theme.dim));
    frame.render_widget(widget, area);
}

/// Human-friendly token count: `1234`, `12.3k`, `1.5M`.
#[inline]
fn format_tokens(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}Mtok", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}ktok", n as f64 / 1_000.0)
    } else {
        format!("{n}tok")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_status_info() {
        let info = StatusInfo::default();
        assert_eq!(info.total_tokens, 0);
        assert_eq!(info.total_cost_usd, 0.0);
    }

    #[test]
    fn format_tokens_small() {
        assert_eq!(format_tokens(500), "500tok");
    }

    #[test]
    fn format_tokens_kilo() {
        assert_eq!(format_tokens(12_345), "12.3ktok");
    }

    #[test]
    fn format_tokens_mega() {
        assert_eq!(format_tokens(1_500_000), "1.5Mtok");
    }

    #[test]
    fn non_zero_cost_format() {
        let info = StatusInfo {
            total_cost_usd: 0.0123,
            ..Default::default()
        };
        let s = if info.total_cost_usd > 0.0 {
            format!("${:.4}", info.total_cost_usd)
        } else {
            "$0.00".into()
        };
        assert_eq!(s, "$0.0123");
    }

    #[test]
    fn zero_cost_format() {
        let info = StatusInfo::default();
        let s = if info.total_cost_usd > 0.0 {
            format!("${:.4}", info.total_cost_usd)
        } else {
            "$0.00".into()
        };
        assert_eq!(s, "$0.00");
    }

    #[test]
    fn default_auth_ok_and_ready() {
        let info = StatusInfo::default();
        assert!(info.auth_ok);
        assert!(info.is_ready);
    }

    #[test]
    fn auth_failure_state() {
        let info = StatusInfo {
            auth_ok: false,
            ..Default::default()
        };
        assert!(!info.auth_ok);
    }

    #[test]
    fn loading_state() {
        let info = StatusInfo {
            is_ready: false,
            ..Default::default()
        };
        assert!(!info.is_ready);
    }
}
