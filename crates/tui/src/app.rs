//! Top-level application state, screen routing, and render dispatch.

use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::onboarding::OnboardingView;
use crate::repl::ReplView;
use crate::theme::{Theme, ThemeName};

/// Which screen the application is currently showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppScreen {
    /// First-time API key setup.
    Onboarding,
    /// Interactive REPL.
    Repl,
    /// Startup / loading splash.
    Loading,
}

/// High-level action emitted by input handling, consumed by the main loop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppAction {
    /// User submitted a prompt.
    Submit(String),
    /// User requested quit.
    Quit,
    /// Scroll messages up.
    ScrollUp,
    /// Scroll messages down.
    ScrollDown,
    /// Accept the active permission dialog.
    AcceptPermission,
    /// Deny the active permission dialog.
    DenyPermission,
    /// No-op.
    Noop,
}

/// Root application state.
#[derive(Debug, Clone)]
pub struct App {
    /// Active screen.
    pub screen: AppScreen,
    /// Set to `true` to exit the main loop.
    pub should_quit: bool,
    /// Monotonic tick counter (drives animations).
    pub tick_count: u64,
    /// REPL sub-view.
    pub repl: ReplView,
    /// Onboarding sub-view.
    pub onboarding: OnboardingView,
    /// Active colour theme.
    pub theme: Theme,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    /// Create a new `App` starting on the Loading screen with a dark theme.
    #[must_use]
    pub fn new() -> Self {
        Self {
            screen: AppScreen::Loading,
            should_quit: false,
            tick_count: 0,
            repl: ReplView::new(),
            onboarding: OnboardingView::new(),
            theme: Theme::from_name(ThemeName::Auto),
        }
    }

    /// Advance animations by one frame.
    #[inline]
    pub fn tick(&mut self) {
        self.tick_count += 1;
        if self.repl.is_loading {
            self.repl.spinner.tick();
        }
    }

    /// Signal that the application should exit.
    #[inline]
    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    /// Draw the current screen into the given `Frame`.
    pub fn render(&self, frame: &mut Frame) {
        let area = frame.area();
        match self.screen {
            AppScreen::Loading => self.render_loading(frame, area),
            AppScreen::Repl => self.repl.render(frame, area, &self.theme),
            AppScreen::Onboarding => self.onboarding.render(frame, area, &self.theme),
        }
    }

    // ── Loading / splash screen ──

    fn render_loading(&self, frame: &mut Frame, area: Rect) {
        let logo = ASCII_LOGO;
        let mut lines: Vec<Line<'_>> = Vec::new();

        for line in logo.lines() {
            lines.push(Line::from(Span::styled(
                line.to_string(),
                Style::default()
                    .fg(self.theme.primary)
                    .add_modifier(Modifier::BOLD),
            )));
        }
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            "Claude Code (Rust) — Loading...".to_string(),
            Style::default().fg(self.theme.dim),
        )));

        let text = Text::from(lines);
        let text_height = text.height() as u16;
        let paragraph = Paragraph::new(text).alignment(Alignment::Center);
        // Vertically centre by adding top padding
        let pad_top = area.height.saturating_sub(text_height) / 2;
        let inner = Rect {
            x: area.x,
            y: area.y + pad_top,
            width: area.width,
            height: area.height.saturating_sub(pad_top),
        };
        frame.render_widget(paragraph, inner);
    }
}

/// ASCII art logo displayed on the loading screen.
const ASCII_LOGO: &str = r"
   ___  _                    _         ___            _
  / __\| |  __ _  _   _   __| |  ___  / __\  ___   __| |  ___
 / /   | | / _` || | | | / _` | / _ \/ /    / _ \ / _` | / _ \
/ /___ | || (_| || |_| || (_| ||  __// /___ | (_) || (_| ||  __/
\____/ |_| \__,_| \__,_| \__,_| \___|\____/  \___/  \__,_| \___|
";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_app_defaults() {
        let app = App::new();
        assert!(!app.should_quit);
        assert_eq!(app.tick_count, 0);
        assert_eq!(app.screen, AppScreen::Loading);
    }

    #[test]
    fn quit_sets_flag() {
        let mut app = App::new();
        app.quit();
        assert!(app.should_quit);
    }

    #[test]
    fn tick_increments() {
        let mut app = App::new();
        app.tick();
        app.tick();
        assert_eq!(app.tick_count, 2);
    }

    #[test]
    fn tick_advances_spinner_when_loading() {
        let mut app = App::new();
        app.screen = AppScreen::Repl;
        app.repl.is_loading = true;
        let before = app.repl.spinner.current_frame().to_string();
        app.tick();
        let after = app.repl.spinner.current_frame().to_string();
        assert_ne!(before, after);
    }

    #[test]
    fn screen_transitions() {
        let mut app = App::new();
        app.screen = AppScreen::Repl;
        assert_eq!(app.screen, AppScreen::Repl);
        app.screen = AppScreen::Onboarding;
        assert_eq!(app.screen, AppScreen::Onboarding);
    }
}
