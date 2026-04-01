//! REPL view — three-panel layout: messages | status bar | input.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::message_view::MessageView;
use crate::prompt_input::PromptInput;
use crate::spinner::Spinner;
use crate::theme::Theme;

/// Composite view that combines message history, a status bar, and
/// a multi-line input prompt.
#[derive(Debug, Clone)]
pub struct ReplView {
    /// Chat history.
    pub messages: MessageView,
    /// Text input widget.
    pub input: PromptInput,
    /// Loading spinner.
    pub spinner: Spinner,
    /// Whether the assistant is generating a response.
    pub is_loading: bool,
}

impl Default for ReplView {
    fn default() -> Self {
        Self::new()
    }
}

impl ReplView {
    /// Create a fresh REPL view.
    #[must_use]
    pub fn new() -> Self {
        Self {
            messages: MessageView::new(),
            input: PromptInput::new(),
            spinner: Spinner::new(),
            is_loading: false,
        }
    }

    /// Render the three-panel layout into `area`.
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let input_height = (self.input.line_count() as u16 + 2).min(8);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),
                Constraint::Length(1),
                Constraint::Length(input_height),
            ])
            .split(area);

        // ── Messages ──
        self.messages.render(frame, chunks[0], theme);

        // ── Status bar ──
        let status = if self.is_loading {
            Line::from(vec![Span::styled(
                self.spinner.render(),
                Style::default().fg(theme.spinner),
            )])
        } else {
            Line::from(vec![
                Span::styled(" Ready ", Style::default().fg(theme.success)),
                Span::raw("| "),
                Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" send | "),
                Span::styled(
                    "Shift+Enter",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw(" newline | "),
                Span::styled("PgUp/PgDn", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" scroll | "),
                Span::styled("Ctrl+C", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" quit"),
            ])
        };
        let status_widget = Paragraph::new(status).style(Style::default().fg(theme.dim));
        frame.render_widget(status_widget, chunks[1]);

        // ── Input box ──
        let input_text = self.input.text();
        let placeholder_style = Style::default().fg(theme.dim);
        let input_block = Block::default()
            .borders(Borders::ALL)
            .title(" > ")
            .border_style(Style::default().fg(theme.border));

        let input_widget = if input_text.is_empty() {
            Paragraph::new(Span::styled(
                "Type a message...",
                placeholder_style,
            ))
            .block(input_block)
        } else {
            Paragraph::new(input_text).block(input_block)
        };
        frame.render_widget(input_widget, chunks[2]);

        // Cursor
        let (crow, ccol) = self.input.cursor();
        #[allow(clippy::cast_possible_truncation)]
        frame.set_cursor_position((
            chunks[2].x + 1 + ccol as u16,
            chunks[2].y + 1 + crow as u16,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_not_loading() {
        let rv = ReplView::new();
        assert!(!rv.is_loading);
        assert!(rv.input.is_empty());
    }

    #[test]
    fn loading_state() {
        let mut rv = ReplView::new();
        rv.is_loading = true;
        rv.spinner.tick();
        assert!(rv.is_loading);
    }

    #[test]
    fn input_starts_empty() {
        let rv = ReplView::new();
        assert!(rv.input.is_empty());
        assert_eq!(rv.input.text(), "");
    }
}
