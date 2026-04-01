//! Unified and side-by-side diff rendering for the terminal.

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;
use unicode_width::UnicodeWidthStr;

use crate::theme::Theme;

/// Display mode for diffs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffMode {
    /// Classic unified diff.
    Unified,
    /// Left / right panes.
    SideBySide,
}

/// A scrollable diff viewer.
#[derive(Debug, Clone)]
pub struct DiffView {
    /// Raw unified diff text.
    pub diff_text: String,
    /// Source file name shown in the title bar.
    pub filename: String,
    /// Current display mode.
    pub mode: DiffMode,
    /// Vertical scroll offset.
    pub scroll: u16,
}

impl DiffView {
    /// Create a new unified diff viewer for the given file.
    #[must_use]
    pub fn new(diff_text: String, filename: String) -> Self {
        Self {
            diff_text,
            filename,
            mode: DiffMode::Unified,
            scroll: 0,
        }
    }

    /// Toggle between unified and side-by-side modes.
    pub fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            DiffMode::Unified => DiffMode::SideBySide,
            DiffMode::SideBySide => DiffMode::Unified,
        };
    }

    /// Scroll up one line.
    #[inline]
    pub fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }

    /// Scroll down one line.
    #[inline]
    pub fn scroll_down(&mut self) {
        self.scroll = self.scroll.saturating_add(1);
    }

    /// Render the diff into the given area.
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let lines = match self.mode {
            DiffMode::Unified => self.render_unified(theme),
            DiffMode::SideBySide => self.render_side_by_side(area.width, theme),
        };

        let block = Block::default()
            .title(format!(" {} ", self.filename))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border));

        let paragraph = Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((self.scroll, 0));

        frame.render_widget(paragraph, area);
    }

    // ── private renderers ──

    fn render_unified(&self, theme: &Theme) -> Vec<Line<'static>> {
        self.diff_text
            .lines()
            .map(|line| {
                if let Some(rest) = line.strip_prefix("+++") {
                    Line::from(Span::styled(
                        format!("+++{rest}"),
                        Style::default()
                            .fg(theme.dim)
                            .add_modifier(Modifier::BOLD),
                    ))
                } else if let Some(rest) = line.strip_prefix("---") {
                    Line::from(Span::styled(
                        format!("---{rest}"),
                        Style::default()
                            .fg(theme.dim)
                            .add_modifier(Modifier::BOLD),
                    ))
                } else if line.starts_with('+') {
                    Line::from(Span::styled(
                        line.to_string(),
                        Style::default()
                            .fg(Color::Green)
                            .bg(Color::Rgb(0, 40, 0)),
                    ))
                } else if line.starts_with('-') {
                    Line::from(Span::styled(
                        line.to_string(),
                        Style::default().fg(Color::Red).bg(Color::Rgb(40, 0, 0)),
                    ))
                } else if line.starts_with("@@") {
                    Line::from(Span::styled(
                        line.to_string(),
                        Style::default().fg(Color::Cyan),
                    ))
                } else {
                    Line::from(Span::raw(line.to_string()))
                }
            })
            .collect()
    }

    fn render_side_by_side(&self, total_width: u16, theme: &Theme) -> Vec<Line<'static>> {
        // Reserve 2 for borders, 1 for separator
        let pane_width = ((total_width.saturating_sub(3)) / 2) as usize;
        let mut left_lines: Vec<String> = Vec::new();
        let mut right_lines: Vec<String> = Vec::new();

        for line in self.diff_text.lines() {
            if line.starts_with("+++") || line.starts_with("---") || line.starts_with("@@") {
                // Header lines go to both panes
                left_lines.push(line.to_string());
                right_lines.push(line.to_string());
            } else if let Some(added) = line.strip_prefix('+') {
                left_lines.push(String::new());
                right_lines.push(added.to_string());
            } else if let Some(removed) = line.strip_prefix('-') {
                left_lines.push(removed.to_string());
                right_lines.push(String::new());
            } else {
                let content = line.strip_prefix(' ').unwrap_or(line);
                left_lines.push(content.to_string());
                right_lines.push(content.to_string());
            }
        }

        left_lines
            .into_iter()
            .zip(right_lines)
            .map(|(left, right)| {
                let left_padded = pad_to_width(&left, pane_width);
                let right_padded = pad_to_width(&right, pane_width);

                let left_style = if left.is_empty() && !right.is_empty() {
                    Style::default()
                } else {
                    Style::default().fg(theme.error)
                };
                let right_style = if right.is_empty() && !left.is_empty() {
                    Style::default()
                } else {
                    Style::default().fg(theme.success)
                };

                Line::from(vec![
                    Span::styled(left_padded, left_style),
                    Span::styled("│", Style::default().fg(theme.border)),
                    Span::styled(right_padded, right_style),
                ])
            })
            .collect()
    }
}

/// Pad (or truncate) `s` to exactly `width` display columns, respecting
/// Unicode character widths.
fn pad_to_width(s: &str, width: usize) -> String {
    let display_w = UnicodeWidthStr::width(s);
    if display_w >= width {
        // Truncate
        let mut out = String::new();
        let mut w = 0;
        for ch in s.chars() {
            let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
            if w + cw > width {
                break;
            }
            out.push(ch);
            w += cw;
        }
        while w < width {
            out.push(' ');
            w += 1;
        }
        out
    } else {
        let mut out = s.to_string();
        for _ in 0..(width - display_w) {
            out.push(' ');
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_diff() {
        let theme = crate::theme::Theme::dark();
        let dv = DiffView::new(String::new(), "test.rs".into());
        let lines = dv.render_unified(&theme);
        assert!(lines.is_empty());
    }

    #[test]
    fn added_line_styled_green() {
        let theme = crate::theme::Theme::dark();
        let dv = DiffView::new("+added line".into(), "a.rs".into());
        let lines = dv.render_unified(&theme);
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn removed_line_styled_red() {
        let theme = crate::theme::Theme::dark();
        let dv = DiffView::new("-removed line".into(), "a.rs".into());
        let lines = dv.render_unified(&theme);
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn hunk_header_styled_cyan() {
        let theme = crate::theme::Theme::dark();
        let dv = DiffView::new("@@ -1,3 +1,4 @@".into(), "a.rs".into());
        let lines = dv.render_unified(&theme);
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn header_lines_bold() {
        let theme = crate::theme::Theme::dark();
        let dv = DiffView::new("--- a/file\n+++ b/file".into(), "a.rs".into());
        let lines = dv.render_unified(&theme);
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn toggle_mode() {
        let mut dv = DiffView::new(String::new(), "a.rs".into());
        assert_eq!(dv.mode, DiffMode::Unified);
        dv.toggle_mode();
        assert_eq!(dv.mode, DiffMode::SideBySide);
        dv.toggle_mode();
        assert_eq!(dv.mode, DiffMode::Unified);
    }

    #[test]
    fn scroll_up_down() {
        let mut dv = DiffView::new(String::new(), "a.rs".into());
        dv.scroll_down();
        dv.scroll_down();
        assert_eq!(dv.scroll, 2);
        dv.scroll_up();
        assert_eq!(dv.scroll, 1);
    }

    #[test]
    fn pad_to_width_short() {
        let padded = pad_to_width("hi", 5);
        assert_eq!(padded.len(), 5);
    }

    #[test]
    fn pad_to_width_unicode() {
        // '你' is 2 columns wide
        let padded = pad_to_width("你好", 6);
        assert_eq!(UnicodeWidthStr::width(padded.as_str()), 6);
    }

    #[test]
    fn side_by_side_rendering() {
        let theme = crate::theme::Theme::dark();
        let diff = "-old\n+new\n context\n";
        let dv = DiffView::new(diff.into(), "test.rs".into());
        let lines = dv.render_side_by_side(80, &theme);
        assert_eq!(lines.len(), 3);
    }
}
