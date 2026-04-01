//! Key-binding helpers and constants.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Returns `true` for quit shortcuts: Ctrl+C, Ctrl+D, bare `q`.
#[inline]
#[must_use]
pub fn is_quit_key(key: &KeyEvent) -> bool {
    matches!(
        (key.code, key.modifiers),
        (KeyCode::Char('c'), m) | (KeyCode::Char('d'), m)
            if m.contains(KeyModifiers::CONTROL)
    ) || key.code == KeyCode::Char('q')
}

/// Returns `true` for the submit (Enter without Shift) shortcut.
#[inline]
#[must_use]
pub fn is_submit_key(key: &KeyEvent) -> bool {
    key.code == KeyCode::Enter && !key.modifiers.contains(KeyModifiers::SHIFT)
}

/// Returns `true` for scroll-up keys: Up, PageUp, Ctrl+Y, Shift+Up.
#[inline]
#[must_use]
pub fn is_scroll_up(key: &KeyEvent) -> bool {
    matches!(key.code, KeyCode::PageUp)
        || (key.code == KeyCode::Up && key.modifiers.contains(KeyModifiers::SHIFT))
        || (key.code == KeyCode::Char('y') && key.modifiers.contains(KeyModifiers::CONTROL))
}

/// Returns `true` for scroll-down keys: Down, PageDown, Ctrl+E, Shift+Down.
#[inline]
#[must_use]
pub fn is_scroll_down(key: &KeyEvent) -> bool {
    matches!(key.code, KeyCode::PageDown)
        || (key.code == KeyCode::Down && key.modifiers.contains(KeyModifiers::SHIFT))
        || (key.code == KeyCode::Char('e') && key.modifiers.contains(KeyModifiers::CONTROL))
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
    fn submit_enter_no_shift() {
        assert!(is_submit_key(&key(KeyCode::Enter, KeyModifiers::empty())));
    }

    #[test]
    fn shift_enter_is_not_submit() {
        assert!(!is_submit_key(&key(KeyCode::Enter, KeyModifiers::SHIFT)));
    }

    #[test]
    fn page_up_is_scroll_up() {
        assert!(is_scroll_up(&key(KeyCode::PageUp, KeyModifiers::empty())));
    }

    #[test]
    fn page_down_is_scroll_down() {
        assert!(is_scroll_down(&key(
            KeyCode::PageDown,
            KeyModifiers::empty()
        )));
    }
}
