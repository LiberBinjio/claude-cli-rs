//! Multi-line text-input widget with cursor movement and command history.
//!
//! `cursor_col` is always a **character index**, not a byte offset. All
//! `String` mutations go through [`char_to_byte_index`] so that multi-byte
//! characters (CJK, emoji, etc.) are handled correctly.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Maximum number of history entries to retain.
const MAX_HISTORY: usize = 100;

/// Convert a character position to a byte offset within `s`.
/// Returns `s.len()` when `char_idx` is past the last character.
#[inline]
fn char_to_byte_index(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(i, _)| i)
        .unwrap_or(s.len())
}

/// Number of characters in `s` (not bytes).
#[inline]
fn char_count(s: &str) -> usize {
    s.chars().count()
}

/// A multi-line editable text buffer with cursor tracking.
#[derive(Debug, Clone)]
pub struct PromptInput {
    lines: Vec<String>,
    cursor_row: usize,
    cursor_col: usize,
    history: Vec<String>,
    history_index: Option<usize>,
}

impl Default for PromptInput {
    fn default() -> Self {
        Self::new()
    }
}

impl PromptInput {
    /// Create an empty input buffer.
    #[must_use]
    pub fn new() -> Self {
        Self {
            lines: vec![String::new()],
            cursor_row: 0,
            cursor_col: 0,
            history: Vec::new(),
            history_index: None,
        }
    }

    /// Concatenated text across all lines.
    #[must_use]
    pub fn text(&self) -> String {
        self.lines.join("\n")
    }

    /// `true` when every line is empty.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lines.iter().all(String::is_empty)
    }

    /// Number of lines in the buffer.
    #[inline]
    #[must_use]
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Current cursor position as `(row, col)`.
    #[inline]
    #[must_use]
    pub fn cursor(&self) -> (usize, usize) {
        (self.cursor_row, self.cursor_col)
    }

    /// Process a key event.  Returns `true` when the user pressed
    /// Enter (without Shift) — i.e. the input should be submitted.
    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        match (key.code, key.modifiers) {
            // Submit
            (KeyCode::Enter, m) if !m.contains(KeyModifiers::SHIFT) => {
                return true;
            }
            // New line
            (KeyCode::Enter, _) => {
                let byte_idx = char_to_byte_index(&self.lines[self.cursor_row], self.cursor_col);
                let rest = self.lines[self.cursor_row].split_off(byte_idx);
                self.cursor_row += 1;
                self.cursor_col = 0;
                self.lines.insert(self.cursor_row, rest);
            }
            // Backspace
            (KeyCode::Backspace, _) => {
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                    let byte_idx =
                        char_to_byte_index(&self.lines[self.cursor_row], self.cursor_col);
                    self.lines[self.cursor_row].remove(byte_idx);
                } else if self.cursor_row > 0 {
                    let line = self.lines.remove(self.cursor_row);
                    self.cursor_row -= 1;
                    self.cursor_col = char_count(&self.lines[self.cursor_row]);
                    self.lines[self.cursor_row].push_str(&line);
                }
            }
            // Delete
            (KeyCode::Delete, _) => {
                if self.cursor_col < char_count(&self.lines[self.cursor_row]) {
                    let byte_idx =
                        char_to_byte_index(&self.lines[self.cursor_row], self.cursor_col);
                    self.lines[self.cursor_row].remove(byte_idx);
                } else if self.cursor_row + 1 < self.lines.len() {
                    let next = self.lines.remove(self.cursor_row + 1);
                    self.lines[self.cursor_row].push_str(&next);
                }
            }
            // Arrows
            (KeyCode::Left, _) => {
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                }
            }
            (KeyCode::Right, _) => {
                if self.cursor_col < char_count(&self.lines[self.cursor_row]) {
                    self.cursor_col += 1;
                }
            }
            (KeyCode::Up, _) => {
                if self.cursor_row > 0 {
                    self.cursor_row -= 1;
                    self.cursor_col =
                        self.cursor_col.min(char_count(&self.lines[self.cursor_row]));
                } else {
                    self.navigate_history_up();
                }
            }
            (KeyCode::Down, _) => {
                if self.cursor_row + 1 < self.lines.len() {
                    self.cursor_row += 1;
                    self.cursor_col =
                        self.cursor_col.min(char_count(&self.lines[self.cursor_row]));
                } else {
                    self.navigate_history_down();
                }
            }
            // Home / End
            (KeyCode::Home, _) => {
                self.cursor_col = 0;
            }
            (KeyCode::End, _) => {
                self.cursor_col = char_count(&self.lines[self.cursor_row]);
            }
            // Printable character
            (KeyCode::Char(c), _) => {
                let byte_idx =
                    char_to_byte_index(&self.lines[self.cursor_row], self.cursor_col);
                self.lines[self.cursor_row].insert(byte_idx, c);
                self.cursor_col += 1;
            }
            _ => {}
        }
        false
    }

    /// Clear the buffer, push non-empty text into history, and return
    /// the submitted text.
    pub fn submit(&mut self) -> String {
        let text = self.text();
        if !text.trim().is_empty() {
            self.history.push(text.clone());
            if self.history.len() > MAX_HISTORY {
                self.history.remove(0);
            }
        }
        self.lines = vec![String::new()];
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.history_index = None;
        text
    }

    // ── history navigation ──

    fn navigate_history_up(&mut self) {
        if self.history.is_empty() {
            return;
        }
        let idx = match self.history_index {
            Some(i) if i > 0 => i - 1,
            Some(_) => return,
            None => self.history.len() - 1,
        };
        self.history_index = Some(idx);
        self.load_history_entry(idx);
    }

    fn navigate_history_down(&mut self) {
        match self.history_index {
            Some(i) if i + 1 < self.history.len() => {
                self.history_index = Some(i + 1);
                self.load_history_entry(i + 1);
            }
            Some(_) => {
                self.history_index = None;
                self.lines = vec![String::new()];
                self.cursor_row = 0;
                self.cursor_col = 0;
            }
            None => {}
        }
    }

    fn load_history_entry(&mut self, idx: usize) {
        self.lines = self.history[idx].split('\n').map(String::from).collect();
        self.cursor_row = self.lines.len() - 1;
        self.cursor_col = char_count(&self.lines[self.cursor_row]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEventKind, KeyEventState};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        }
    }

    fn shift_key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::SHIFT,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        }
    }

    #[test]
    fn type_chars() {
        let mut inp = PromptInput::new();
        inp.handle_key(key(KeyCode::Char('h')));
        inp.handle_key(key(KeyCode::Char('i')));
        assert_eq!(inp.text(), "hi");
    }

    #[test]
    fn enter_submits() {
        let mut inp = PromptInput::new();
        inp.handle_key(key(KeyCode::Char('x')));
        assert!(inp.handle_key(key(KeyCode::Enter)));
    }

    #[test]
    fn shift_enter_newline() {
        let mut inp = PromptInput::new();
        inp.handle_key(key(KeyCode::Char('a')));
        inp.handle_key(shift_key(KeyCode::Enter));
        inp.handle_key(key(KeyCode::Char('b')));
        assert_eq!(inp.text(), "a\nb");
        assert_eq!(inp.line_count(), 2);
    }

    #[test]
    fn backspace_within_line() {
        let mut inp = PromptInput::new();
        inp.handle_key(key(KeyCode::Char('a')));
        inp.handle_key(key(KeyCode::Char('b')));
        inp.handle_key(key(KeyCode::Backspace));
        assert_eq!(inp.text(), "a");
    }

    #[test]
    fn backspace_merges_lines() {
        let mut inp = PromptInput::new();
        inp.handle_key(key(KeyCode::Char('a')));
        inp.handle_key(shift_key(KeyCode::Enter));
        inp.handle_key(key(KeyCode::Char('b')));
        inp.handle_key(key(KeyCode::Home)); // col=0 on line 1
        inp.handle_key(key(KeyCode::Backspace)); // merge
        assert_eq!(inp.text(), "ab");
    }

    #[test]
    fn delete_within_line() {
        let mut inp = PromptInput::new();
        inp.handle_key(key(KeyCode::Char('a')));
        inp.handle_key(key(KeyCode::Char('b')));
        inp.handle_key(key(KeyCode::Home));
        inp.handle_key(key(KeyCode::Delete));
        assert_eq!(inp.text(), "b");
    }

    #[test]
    fn submit_clears_and_stores_history() {
        let mut inp = PromptInput::new();
        inp.handle_key(key(KeyCode::Char('x')));
        let txt = inp.submit();
        assert_eq!(txt, "x");
        assert!(inp.is_empty());
        assert_eq!(inp.line_count(), 1);
    }

    #[test]
    fn history_nav() {
        let mut inp = PromptInput::new();
        inp.handle_key(key(KeyCode::Char('a')));
        inp.submit();
        inp.handle_key(key(KeyCode::Char('b')));
        inp.submit();
        // Up → should load "b"
        inp.handle_key(key(KeyCode::Up));
        assert_eq!(inp.text(), "b");
        // Up again → "a"
        inp.handle_key(key(KeyCode::Up));
        assert_eq!(inp.text(), "a");
        // Down → back to "b"
        inp.handle_key(key(KeyCode::Down));
        assert_eq!(inp.text(), "b");
    }

    #[test]
    fn history_capped() {
        let mut inp = PromptInput::new();
        for i in 0..MAX_HISTORY + 20 {
            inp.handle_key(key(KeyCode::Char('a')));
            inp.handle_key(key(KeyCode::Char(
                char::from_digit((i % 10) as u32, 10).unwrap_or('0'),
            )));
            inp.submit();
        }
        assert!(inp.history.len() <= MAX_HISTORY);
    }

    #[test]
    fn cursor_movement() {
        let mut inp = PromptInput::new();
        inp.handle_key(key(KeyCode::Char('a')));
        inp.handle_key(key(KeyCode::Char('b')));
        inp.handle_key(key(KeyCode::Left));
        assert_eq!(inp.cursor(), (0, 1));
        inp.handle_key(key(KeyCode::Right));
        assert_eq!(inp.cursor(), (0, 2));
    }

    #[test]
    fn chinese_input() {
        let mut inp = PromptInput::new();
        inp.handle_key(key(KeyCode::Char('你')));
        inp.handle_key(key(KeyCode::Char('好')));
        assert_eq!(inp.text(), "你好");
        assert_eq!(inp.cursor(), (0, 2));
    }

    #[test]
    fn mixed_ascii_chinese() {
        let mut inp = PromptInput::new();
        inp.handle_key(key(KeyCode::Char('a')));
        inp.handle_key(key(KeyCode::Char('中')));
        inp.handle_key(key(KeyCode::Char('b')));
        assert_eq!(inp.text(), "a中b");
        assert_eq!(inp.cursor(), (0, 3));
    }

    #[test]
    fn backspace_chinese() {
        let mut inp = PromptInput::new();
        inp.handle_key(key(KeyCode::Char('你')));
        inp.handle_key(key(KeyCode::Char('好')));
        inp.handle_key(key(KeyCode::Backspace));
        assert_eq!(inp.text(), "你");
        assert_eq!(inp.cursor(), (0, 1));
    }

    #[test]
    fn delete_chinese() {
        let mut inp = PromptInput::new();
        inp.handle_key(key(KeyCode::Char('你')));
        inp.handle_key(key(KeyCode::Char('好')));
        inp.handle_key(key(KeyCode::Home));
        inp.handle_key(key(KeyCode::Delete));
        assert_eq!(inp.text(), "好");
    }

    #[test]
    fn emoji_input() {
        let mut inp = PromptInput::new();
        inp.handle_key(key(KeyCode::Char('🎉')));
        inp.handle_key(key(KeyCode::Char('a')));
        assert_eq!(inp.text(), "🎉a");
        assert_eq!(inp.cursor(), (0, 2));
    }

    #[test]
    fn cursor_movement_chinese() {
        let mut inp = PromptInput::new();
        inp.handle_key(key(KeyCode::Char('你')));
        inp.handle_key(key(KeyCode::Char('好')));
        inp.handle_key(key(KeyCode::Char('世')));
        inp.handle_key(key(KeyCode::Left));
        inp.handle_key(key(KeyCode::Left));
        assert_eq!(inp.cursor(), (0, 1));
        // Insert at position 1
        inp.handle_key(key(KeyCode::Char('X')));
        assert_eq!(inp.text(), "你X好世");
    }

    #[test]
    fn shift_enter_with_chinese() {
        let mut inp = PromptInput::new();
        inp.handle_key(key(KeyCode::Char('你')));
        inp.handle_key(shift_key(KeyCode::Enter));
        inp.handle_key(key(KeyCode::Char('好')));
        assert_eq!(inp.text(), "你\n好");
        assert_eq!(inp.line_count(), 2);
    }
}
