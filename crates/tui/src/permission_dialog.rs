//! Modal permission-confirmation dialog (Allow / Deny / Always Allow).

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Alignment, Constraint, Flex, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::theme::Theme;

/// Outcome the user chose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionChoice {
    /// Permit this one invocation.
    Allow,
    /// Block this invocation.
    Deny,
    /// Permit all future invocations of this tool.
    AlwaysAllow,
    /// User hasn't decided yet.
    Pending,
}

/// A centred modal asking the user to approve a tool invocation.
#[derive(Debug, Clone)]
pub struct PermissionDialog {
    /// Tool being invoked.
    pub tool_name: String,
    /// Human-readable summary.
    pub description: String,
    /// Extra detail (e.g. file path, command).
    pub details: String,
    /// Current decision.
    pub choice: PermissionChoice,
    /// Which button is highlighted (0=Allow, 1=Deny, 2=Always).
    pub selected: usize,
}

impl PermissionDialog {
    /// Create a dialog for the given tool invocation.
    #[must_use]
    pub fn new(tool_name: String, description: String, details: String) -> Self {
        Self {
            tool_name,
            description,
            details,
            choice: PermissionChoice::Pending,
            selected: 0,
        }
    }

    /// Process a key event.
    pub fn handle_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Left | KeyCode::Char('h') => {
                self.selected = self.selected.saturating_sub(1);
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.selected = (self.selected + 1).min(2);
            }
            KeyCode::Tab => {
                self.selected = (self.selected + 1) % 3;
            }
            KeyCode::BackTab => {
                self.selected = if self.selected == 0 { 2 } else { self.selected - 1 };
            }
            KeyCode::Enter | KeyCode::Char('y') => {
                self.choice = match self.selected {
                    0 => PermissionChoice::Allow,
                    1 => PermissionChoice::Deny,
                    _ => PermissionChoice::AlwaysAllow,
                };
            }
            KeyCode::Char('n') => {
                self.choice = PermissionChoice::Deny;
            }
            KeyCode::Char('a') => {
                self.choice = PermissionChoice::AlwaysAllow;
            }
            KeyCode::Esc => {
                self.choice = PermissionChoice::Deny;
            }
            _ => {}
        }
    }

    /// `true` once the user has made a choice.
    #[inline]
    #[must_use]
    pub fn is_resolved(&self) -> bool {
        !matches!(self.choice, PermissionChoice::Pending)
    }

    /// Render the dialog as a centred popup.
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let popup_width = 60u16.min(area.width.saturating_sub(4));
        let popup_height = 12u16.min(area.height.saturating_sub(4));
        let popup_area = centered_rect(popup_width, popup_height, area);

        frame.render_widget(Clear, popup_area);

        let mut lines = vec![
            Line::from(Span::styled(
                format!("\u{1f512} {}", self.tool_name),
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::raw(""),
            Line::from(self.description.clone()),
            Line::raw(""),
        ];

        if !self.details.is_empty() {
            lines.push(Line::from(Span::styled(
                self.details.clone(),
                Style::default().fg(theme.dim),
            )));
            lines.push(Line::raw(""));
        }

        lines.push(Line::from(vec![
            button_span("(y) Allow", self.selected == 0, theme),
            Span::raw("  "),
            button_span("(n) Deny", self.selected == 1, theme),
            Span::raw("  "),
            button_span("(a) Always", self.selected == 2, theme),
        ]));

        let block = Block::default()
            .title(" Permission Required ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.warning));

        let paragraph = Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false })
            .alignment(Alignment::Center);

        frame.render_widget(paragraph, popup_area);
    }
}

/// Render a button label, highlighted when selected.
fn button_span(label: &str, selected: bool, theme: &Theme) -> Span<'static> {
    if selected {
        Span::styled(
            label.to_string(),
            Style::default()
                .fg(Color::Black)
                .bg(theme.primary)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(label.to_string(), Style::default().fg(theme.fg))
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
    fn starts_pending() {
        let d = PermissionDialog::new("bash".into(), "run ls".into(), String::new());
        assert!(!d.is_resolved());
        assert_eq!(d.choice, PermissionChoice::Pending);
    }

    #[test]
    fn y_allows() {
        let mut d = PermissionDialog::new("bash".into(), "".into(), String::new());
        d.handle_key(key(KeyCode::Char('y')));
        assert!(d.is_resolved());
        assert_eq!(d.choice, PermissionChoice::Allow);
    }

    #[test]
    fn n_denies() {
        let mut d = PermissionDialog::new("bash".into(), "".into(), String::new());
        d.handle_key(key(KeyCode::Char('n')));
        assert_eq!(d.choice, PermissionChoice::Deny);
    }

    #[test]
    fn a_always_allows() {
        let mut d = PermissionDialog::new("bash".into(), "".into(), String::new());
        d.handle_key(key(KeyCode::Char('a')));
        assert_eq!(d.choice, PermissionChoice::AlwaysAllow);
    }

    #[test]
    fn esc_denies() {
        let mut d = PermissionDialog::new("bash".into(), "".into(), String::new());
        d.handle_key(key(KeyCode::Esc));
        assert_eq!(d.choice, PermissionChoice::Deny);
    }

    #[test]
    fn tab_cycles() {
        let mut d = PermissionDialog::new("bash".into(), "".into(), String::new());
        assert_eq!(d.selected, 0);
        d.handle_key(key(KeyCode::Tab));
        assert_eq!(d.selected, 1);
        d.handle_key(key(KeyCode::Tab));
        assert_eq!(d.selected, 2);
        d.handle_key(key(KeyCode::Tab));
        assert_eq!(d.selected, 0);
    }

    #[test]
    fn backtab_cycles_reverse() {
        let mut d = PermissionDialog::new("bash".into(), "".into(), String::new());
        d.handle_key(key(KeyCode::BackTab));
        assert_eq!(d.selected, 2);
        d.handle_key(key(KeyCode::BackTab));
        assert_eq!(d.selected, 1);
    }

    #[test]
    fn left_right_navigation() {
        let mut d = PermissionDialog::new("bash".into(), "".into(), String::new());
        d.handle_key(key(KeyCode::Right));
        assert_eq!(d.selected, 1);
        d.handle_key(key(KeyCode::Left));
        assert_eq!(d.selected, 0);
        // Can't go below 0
        d.handle_key(key(KeyCode::Left));
        assert_eq!(d.selected, 0);
    }

    #[test]
    fn enter_selects_current() {
        let mut d = PermissionDialog::new("bash".into(), "".into(), String::new());
        d.handle_key(key(KeyCode::Right)); // Deny
        d.handle_key(key(KeyCode::Enter));
        assert_eq!(d.choice, PermissionChoice::Deny);
    }

    #[test]
    fn centered_rect_fits() {
        let area = Rect::new(0, 0, 80, 24);
        let r = centered_rect(60, 12, area);
        assert!(r.x + r.width <= area.width);
        assert!(r.y + r.height <= area.height);
        assert_eq!(r.width, 60);
        assert_eq!(r.height, 12);
    }
}
