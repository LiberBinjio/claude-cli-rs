//! Async event loop — polls crossterm for keyboard/mouse/resize events
//! and emits periodic [`Event::Tick`]s.

use crossterm::event::{self as ct_event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;
use tokio::sync::mpsc;

/// Application event produced by the [`EventLoop`].
#[derive(Debug, Clone)]
pub enum Event {
    /// A key press.
    Key(KeyEvent),
    /// A mouse event.
    Mouse(ct_event::MouseEvent),
    /// Terminal resized to `(cols, rows)`.
    Resize(u16, u16),
    /// Periodic tick (used for spinner animations, etc.).
    Tick,
}

/// Spawns a background tokio task that polls crossterm and sends [`Event`]s
/// through an unbounded channel.
pub struct EventLoop {
    rx: mpsc::UnboundedReceiver<Event>,
}

impl EventLoop {
    /// Create a new event loop with the given tick interval.
    /// A tokio task is spawned immediately to start polling.
    #[must_use]
    pub fn new(tick_rate: Duration) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        tokio::spawn(Self::poll_loop(tx, tick_rate));
        Self { rx }
    }

    /// Receive the next event, or `None` if the channel closed.
    pub async fn next(&mut self) -> Option<Event> {
        self.rx.recv().await
    }

    /// Internal polling loop — runs on a spawned task.
    async fn poll_loop(tx: mpsc::UnboundedSender<Event>, tick_rate: Duration) {
        let mut tick_interval = tokio::time::interval(tick_rate);
        loop {
            let crossterm_event = tokio::task::spawn_blocking(|| {
                if ct_event::poll(Duration::from_millis(50)).unwrap_or(false) {
                    ct_event::read().ok()
                } else {
                    None
                }
            });

            tokio::select! {
                _ = tick_interval.tick() => {
                    if tx.send(Event::Tick).is_err() { break; }
                }
                result = crossterm_event => {
                    if let Ok(Some(evt)) = result {
                        let mapped = match evt {
                            ct_event::Event::Key(k) => Some(Event::Key(k)),
                            ct_event::Event::Mouse(m) => Some(Event::Mouse(m)),
                            ct_event::Event::Resize(w, h) => Some(Event::Resize(w, h)),
                            _ => None,
                        };
                        if let Some(e) = mapped {
                            if tx.send(e).is_err() { break; }
                        }
                    }
                }
            }
        }
    }
}

/// Returns `true` if the key event is a quit shortcut (Ctrl+C, Ctrl+D, or bare `q`).
#[inline]
#[must_use]
pub fn is_quit_key(key: &KeyEvent) -> bool {
    matches!(
        (key.code, key.modifiers),
        (KeyCode::Char('c'), m) | (KeyCode::Char('d'), m)
            if m.contains(KeyModifiers::CONTROL)
    ) || key.code == KeyCode::Char('q')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEventKind, KeyEventState};

    fn key(code: KeyCode, mods: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: mods,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        }
    }

    #[test]
    fn ctrl_c_is_quit() {
        assert!(is_quit_key(&key(KeyCode::Char('c'), KeyModifiers::CONTROL)));
    }

    #[test]
    fn ctrl_d_is_quit() {
        assert!(is_quit_key(&key(KeyCode::Char('d'), KeyModifiers::CONTROL)));
    }

    #[test]
    fn bare_q_is_quit() {
        assert!(is_quit_key(&key(KeyCode::Char('q'), KeyModifiers::empty())));
    }

    #[test]
    fn regular_key_not_quit() {
        assert!(!is_quit_key(&key(KeyCode::Char('a'), KeyModifiers::empty())));
    }
}
