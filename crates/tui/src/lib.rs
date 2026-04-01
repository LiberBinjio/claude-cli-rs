//! Claude Code — terminal UI (ratatui + crossterm)
//!
//! Provides the TUI layer: terminal management, event loop, theming,
//! REPL view, diff viewer, permission dialogs, markdown rendering,
//! status line, and onboarding flow.

pub mod app;
pub mod diff_view;
pub mod event;
pub mod keybindings;
pub mod markdown_render;
pub mod message_view;
pub mod onboarding;
pub mod permission_dialog;
pub mod prompt_input;
pub mod repl;
pub mod spinner;
pub mod status_line;
pub mod terminal;
pub mod theme;
