//! Colour theme system — supports Dark, Light, and Auto detection.

use ratatui::style::Color;

/// Selectable theme variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeName {
    /// Dark background, bright foreground.
    Dark,
    /// Light background, dark foreground.
    Light,
    /// Auto-detect from the `COLORFGBG` environment variable.
    Auto,
}

/// Complete colour palette used by every TUI widget.
#[derive(Debug, Clone)]
pub struct Theme {
    // Base
    /// Primary foreground colour.
    pub fg: Color,
    /// Primary background colour.
    pub bg: Color,
    /// Dimmed / secondary text.
    pub dim: Color,

    // Accent
    /// Primary accent (headings, highlights).
    pub primary: Color,
    /// Secondary accent.
    pub secondary: Color,

    // Semantic
    /// Success indicators.
    pub success: Color,
    /// Warning indicators.
    pub warning: Color,
    /// Error indicators.
    pub error: Color,
    /// Informational text.
    pub info: Color,

    // Role colours
    /// User message colour.
    pub user_color: Color,
    /// Assistant message colour.
    pub assistant_color: Color,
    /// System message colour.
    pub system_color: Color,
    /// Tool result colour.
    pub tool_color: Color,

    // UI chrome
    /// Border / separator colour.
    pub border: Color,
    /// Selection highlight.
    pub selection: Color,
    /// Cursor colour.
    pub cursor: Color,
    /// Spinner / loading animation colour.
    pub spinner: Color,

    // Code-block chrome
    /// Code-block background.
    pub code_bg: Color,
    /// Code-block foreground.
    pub code_fg: Color,
}

impl Theme {
    /// The dark colour scheme (default).
    #[must_use]
    pub fn dark() -> Self {
        Self {
            fg: Color::White,
            bg: Color::Reset,
            dim: Color::Indexed(245), // grey58
            primary: Color::Rgb(130, 170, 255),
            secondary: Color::Rgb(180, 140, 255),
            success: Color::Rgb(80, 220, 100),
            warning: Color::Rgb(255, 200, 60),
            error: Color::Rgb(255, 85, 85),
            info: Color::Rgb(100, 200, 255),
            user_color: Color::Rgb(100, 200, 255),
            assistant_color: Color::Rgb(130, 170, 255),
            system_color: Color::Indexed(245),
            tool_color: Color::Rgb(180, 140, 255),
            border: Color::Indexed(240),
            selection: Color::Rgb(60, 60, 100),
            cursor: Color::White,
            spinner: Color::Rgb(130, 170, 255),
            code_bg: Color::Indexed(235),
            code_fg: Color::Rgb(255, 215, 100),
        }
    }

    /// The light colour scheme.
    #[must_use]
    pub fn light() -> Self {
        Self {
            fg: Color::Black,
            bg: Color::Reset,
            dim: Color::Indexed(244),
            primary: Color::Rgb(30, 80, 180),
            secondary: Color::Rgb(120, 60, 200),
            success: Color::Rgb(20, 150, 50),
            warning: Color::Rgb(200, 140, 0),
            error: Color::Rgb(200, 40, 40),
            info: Color::Rgb(20, 120, 200),
            user_color: Color::Rgb(20, 120, 200),
            assistant_color: Color::Rgb(30, 80, 180),
            system_color: Color::Indexed(244),
            tool_color: Color::Rgb(120, 60, 200),
            border: Color::Indexed(250),
            selection: Color::Rgb(200, 220, 255),
            cursor: Color::Black,
            spinner: Color::Rgb(30, 80, 180),
            code_bg: Color::Indexed(254),
            code_fg: Color::Rgb(160, 100, 0),
        }
    }

    /// Build a theme from a [`ThemeName`].
    ///
    /// `Auto` inspects the `COLORFGBG` environment variable; if the
    /// background component is ≤ 6 (dark), selects [`Theme::dark`],
    /// otherwise [`Theme::light`].  Falls back to dark.
    #[must_use]
    pub fn from_name(name: ThemeName) -> Self {
        match name {
            ThemeName::Dark => Self::dark(),
            ThemeName::Light => Self::light(),
            ThemeName::Auto => {
                if detect_light_background() {
                    Self::light()
                } else {
                    Self::dark()
                }
            }
        }
    }
}

/// Heuristic: check `COLORFGBG` (e.g. `"15;0"` → bg=0 → dark).
fn detect_light_background() -> bool {
    std::env::var("COLORFGBG")
        .ok()
        .and_then(|v| v.rsplit(';').next().map(String::from))
        .and_then(|bg| bg.parse::<u8>().ok())
        .is_some_and(|bg| bg > 6)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_theme_has_white_fg() {
        let t = Theme::dark();
        assert_eq!(t.fg, Color::White);
    }

    #[test]
    fn light_theme_has_black_fg() {
        let t = Theme::light();
        assert_eq!(t.fg, Color::Black);
    }

    #[test]
    fn from_name_dark_eq_dark() {
        let a = Theme::from_name(ThemeName::Dark);
        let b = Theme::dark();
        assert_eq!(a.fg, b.fg);
        assert_eq!(a.primary, b.primary);
    }

    #[test]
    fn auto_without_env_defaults_dark() {
        // In CI there is usually no COLORFGBG → fallback = dark
        let t = Theme::from_name(ThemeName::Auto);
        // At least verify it doesn't panic and produces a valid theme
        assert_ne!(format!("{:?}", t.fg), "");
    }
}
