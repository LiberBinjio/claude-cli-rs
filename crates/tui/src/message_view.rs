//! Scrollable message list with role-coloured labels and streaming support.

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::theme::Theme;

/// Displayable chat message (UI-layer representation).
#[derive(Debug, Clone)]
pub struct DisplayMessage {
    /// Who sent the message.
    pub role: MessageRole,
    /// Full message body.
    pub text: String,
    /// Optional tool invocation metadata.
    pub tool_info: Option<String>,
    /// Unix timestamp (seconds).
    pub timestamp: f64,
}

/// Participant role — used to pick label and colour.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageRole {
    /// Human user.
    User,
    /// Claude assistant.
    Assistant,
    /// System notification.
    System,
    /// Tool execution result.
    ToolResult,
}

/// Scrollable message view with streaming text support.
#[derive(Debug, Clone)]
pub struct MessageView {
    /// Committed messages.
    pub messages: Vec<DisplayMessage>,
    /// Vertical scroll offset (0 = bottom-most).
    pub scroll_offset: u16,
    /// Partial text still being streamed in.
    pub streaming_text: String,
}

impl Default for MessageView {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageView {
    /// Create an empty message view.
    #[must_use]
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            scroll_offset: 0,
            streaming_text: String::new(),
        }
    }

    /// Append a fully-formed message.
    pub fn push(&mut self, msg: DisplayMessage) {
        self.messages.push(msg);
    }

    /// Append partial text to the active stream.
    pub fn append_streaming(&mut self, text: &str) {
        self.streaming_text.push_str(text);
    }

    /// Commit the current streaming text as a completed assistant message.
    pub fn finish_streaming(&mut self) {
        if !self.streaming_text.is_empty() {
            self.messages.push(DisplayMessage {
                role: MessageRole::Assistant,
                text: std::mem::take(&mut self.streaming_text),
                tool_info: None,
                timestamp: 0.0,
            });
        }
    }

    /// Scroll up by `amount` lines.
    #[inline]
    pub fn scroll_up(&mut self, amount: u16) {
        self.scroll_offset = self.scroll_offset.saturating_add(amount);
    }

    /// Scroll down by `amount` lines (towards the bottom).
    #[inline]
    pub fn scroll_down(&mut self, amount: u16) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
    }

    /// Jump to the newest messages.
    #[inline]
    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
    }

    /// Render the message list into the given area.
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let mut lines: Vec<Line<'_>> = Vec::new();

        for msg in &self.messages {
            let (prefix, color) = match msg.role {
                MessageRole::User => ("You", theme.user_color),
                MessageRole::Assistant => ("Claude", theme.assistant_color),
                MessageRole::System => ("System", theme.system_color),
                MessageRole::ToolResult => ("Tool", theme.tool_color),
            };

            lines.push(Line::from(vec![Span::styled(
                format!("{prefix}: "),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            )]));

            for text_line in msg.text.lines() {
                lines.push(Line::from(format!("  {text_line}")));
            }

            if let Some(ref info) = msg.tool_info {
                lines.push(Line::from(Span::styled(
                    format!("  [{info}]"),
                    Style::default().fg(theme.dim),
                )));
            }

            lines.push(Line::raw(""));
        }

        // Streaming text (still being generated)
        if !self.streaming_text.is_empty() {
            lines.push(Line::from(vec![Span::styled(
                "Claude: ",
                Style::default()
                    .fg(theme.assistant_color)
                    .add_modifier(Modifier::BOLD),
            )]));
            for text_line in self.streaming_text.lines() {
                lines.push(Line::from(format!("  {text_line}")));
            }
        }

        let text = Text::from(lines);
        let paragraph = Paragraph::new(text)
            .block(Block::default().borders(Borders::NONE))
            .wrap(Wrap { trim: false })
            .scroll((self.scroll_offset, 0));
        frame.render_widget(paragraph, area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_and_count() {
        let mut mv = MessageView::new();
        mv.push(DisplayMessage {
            role: MessageRole::User,
            text: "hello".into(),
            tool_info: None,
            timestamp: 0.0,
        });
        assert_eq!(mv.messages.len(), 1);
    }

    #[test]
    fn streaming_append_and_finish() {
        let mut mv = MessageView::new();
        mv.append_streaming("part1 ");
        mv.append_streaming("part2");
        mv.finish_streaming();
        assert_eq!(mv.messages.len(), 1);
        assert_eq!(mv.messages[0].text, "part1 part2");
        assert!(mv.streaming_text.is_empty());
    }

    #[test]
    fn scroll_up_down() {
        let mut mv = MessageView::new();
        mv.scroll_up(5);
        assert_eq!(mv.scroll_offset, 5);
        mv.scroll_down(3);
        assert_eq!(mv.scroll_offset, 2);
        mv.scroll_to_bottom();
        assert_eq!(mv.scroll_offset, 0);
    }

    #[test]
    fn finish_empty_stream_is_noop() {
        let mut mv = MessageView::new();
        mv.finish_streaming();
        assert!(mv.messages.is_empty());
    }
}
