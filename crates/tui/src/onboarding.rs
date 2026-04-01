//! First-time onboarding view — API key entry and welcome text.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Alignment, Constraint, Flex, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::theme::Theme;

/// Onboarding wizard shown on first launch.
#[derive(Debug, Clone)]
pub struct OnboardingView {
    /// Current wizard step (reserved for future multi-step flow).
    pub step: usize,
    /// Text buffer for the API key input field.
    pub api_key_input: String,
}

impl Default for OnboardingView {
    fn default() -> Self {
        Self::new()
    }
}

impl OnboardingView {
    /// Create an empty onboarding view.
    #[must_use]
    pub fn new() -> Self {
        Self {
            step: 0,
            api_key_input: String::new(),
        }
    }

    /// Handle a key event.  Returns `true` when the user presses Enter
    /// to submit the key.
    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char(c) => {
                self.api_key_input.push(c);
            }
            KeyCode::Backspace => {
                self.api_key_input.pop();
            }
            KeyCode::Enter => {
                return true;
            }
            _ => {}
        }
        false
    }

    /// Render the onboarding screen.
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let masked = if self.api_key_input.is_empty() {
            "sk-ant-...".to_string()
        } else {
            // Show first 7 chars, mask the rest
            let visible = self.api_key_input.chars().take(7).collect::<String>();
            let rest_len = self.api_key_input.len().saturating_sub(7);
            format!("{visible}{}", "*".repeat(rest_len))
        };

        let key_style = if self.api_key_input.is_empty() {
            Style::default().fg(theme.dim)
        } else {
            Style::default().fg(theme.fg)
        };

        let lines = vec![
            Line::from(Span::styled(
                "Welcome to Claude Code (Rust)! \u{1f980}",
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::raw(""),
            Line::raw("To get started, you need an Anthropic API key."),
            Line::raw(""),
            Line::raw("Option 1: Set ANTHROPIC_API_KEY environment variable"),
            Line::raw("Option 2: Enter your API key below"),
            Line::raw(""),
            Line::from(vec![
                Span::raw("API Key: "),
                Span::styled(masked, key_style),
            ]),
            Line::raw(""),
            Line::from(Span::styled(
                "Press Enter to continue \u{00b7} Esc to quit",
                Style::default().fg(theme.dim),
            )),
        ];

        let text = Text::from(lines);
        let height = (text.height() as u16 + 2).min(area.height.saturating_sub(4));
        let width = 60u16.min(area.width.saturating_sub(4));
        let centered = centered_rect(width, height, area);

        let block = Block::default()
            .title(" First Time Setup ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.primary));

        let paragraph = Paragraph::new(text)
            .block(block)
            .wrap(Wrap { trim: false })
            .alignment(Alignment::Left);

        frame.render_widget(paragraph, centered);
    }
}

/// Compute a centred rectangle of the given size within `area`.
fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let v = Layout::vertical([Constraint::Length(height)])
        .flex(Flex::Center)
        .split(area);
    let h = Layout::horizontal([Constraint::Length(width)])
        .flex(Flex::Center)
        .split(v[0]);
    h[0]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEventKind, KeyEventState, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        }
    }

    #[test]
    fn default_empty_key() {
        let v = OnboardingView::new();
        assert!(v.api_key_input.is_empty());
    }

    #[test]
    fn typing_chars() {
        let mut v = OnboardingView::new();
        v.handle_key(key(KeyCode::Char('s')));
        v.handle_key(key(KeyCode::Char('k')));
        assert_eq!(v.api_key_input, "sk");
    }

    #[test]
    fn backspace_removes() {
        let mut v = OnboardingView::new();
        v.handle_key(key(KeyCode::Char('a')));
        v.handle_key(key(KeyCode::Backspace));
        assert!(v.api_key_input.is_empty());
    }

    #[test]
    fn enter_submits() {
        let mut v = OnboardingView::new();
        v.handle_key(key(KeyCode::Char('x')));
        assert!(v.handle_key(key(KeyCode::Enter)));
    }
}
