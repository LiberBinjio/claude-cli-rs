//! Braille-frame spinner for loading animations.

/// Braille animation frames for the spinner.
const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// A simple text spinner with a configurable message.
#[derive(Debug, Clone)]
pub struct Spinner {
    frame: usize,
    /// The label displayed alongside the spinner glyph.
    pub message: String,
}

impl Default for Spinner {
    fn default() -> Self {
        Self::new()
    }
}

impl Spinner {
    /// Create a spinner with the default "Thinking…" message.
    #[must_use]
    pub fn new() -> Self {
        Self {
            frame: 0,
            message: "Thinking...".to_string(),
        }
    }

    /// Create a spinner with a custom message.
    #[must_use]
    pub fn with_message(msg: &str) -> Self {
        Self {
            frame: 0,
            message: msg.to_string(),
        }
    }

    /// Advance to the next animation frame.
    #[inline]
    pub fn tick(&mut self) {
        self.frame = (self.frame + 1) % SPINNER_FRAMES.len();
    }

    /// The current braille glyph.
    #[inline]
    #[must_use]
    pub fn current_frame(&self) -> &str {
        SPINNER_FRAMES[self.frame]
    }

    /// Render as `"⠋ Thinking..."`.
    #[inline]
    #[must_use]
    pub fn render(&self) -> String {
        format!("{} {}", self.current_frame(), self.message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_starts_at_frame_zero() {
        let s = Spinner::new();
        assert_eq!(s.current_frame(), "⠋");
    }

    #[test]
    fn tick_advances_frame() {
        let mut s = Spinner::new();
        s.tick();
        assert_eq!(s.current_frame(), "⠙");
    }

    #[test]
    fn cycles_after_full_rotation() {
        let mut s = Spinner::new();
        for _ in 0..SPINNER_FRAMES.len() {
            s.tick();
        }
        assert_eq!(s.current_frame(), SPINNER_FRAMES[0]);
    }

    #[test]
    fn custom_message() {
        let s = Spinner::with_message("Loading...");
        assert!(s.render().contains("Loading..."));
    }

    #[test]
    fn render_contains_frame() {
        let s = Spinner::new();
        assert!(s.render().starts_with('⠋'));
    }
}
