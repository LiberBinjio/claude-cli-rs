//! Terminal initialization, restoration, and panic-safety.

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;

/// Convenience alias for the ratatui terminal backed by crossterm on stdout.
pub type Tui = Terminal<CrosstermBackend<io::Stdout>>;

/// Enter raw mode, switch to alternate screen, enable mouse capture,
/// and return the configured [`Tui`].
pub fn init() -> anyhow::Result<Tui> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

/// Restore the terminal to its original state (reverse of [`init`]).
pub fn restore() -> anyhow::Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}

/// Install a panic hook that restores the terminal before printing the default
/// panic message, preventing a garbled terminal on crash.
pub fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // Best-effort restore — ignore errors
        let _ = restore();
        default_hook(info);
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_panic_hook_runs_without_panic() {
        // Just verifying it doesn't panic or error at setup time.
        // We can't actually trigger a panic in tests safely, but calling it
        // twice should also be fine.
        install_panic_hook();
    }

    #[test]
    fn restore_without_init_is_harmless() {
        // restore() with no prior init should not panic (may return error).
        let _ = restore();
    }
}
