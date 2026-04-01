//! Lightweight Markdown → `Vec<Line>` renderer for terminal display.
//!
//! Handles headings, fenced code blocks, bullet lists, blockquotes,
//! horizontal rules, and inline **bold** / *italic* / `code` spans.

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::theme::Theme;

/// Convert a Markdown string into styled ratatui [`Line`]s.
#[must_use]
pub fn render_markdown(text: &str, theme: &Theme) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut in_code_block = false;
    let mut code_lang = String::new();
    let _ = &code_lang; // used in header formatting below

    for raw_line in text.lines() {
        // ── fenced code block toggle ──
        if raw_line.starts_with("```") {
            if in_code_block {
                in_code_block = false;
                lines.push(Line::from(Span::styled(
                    "└──────────────────────────────────────────┘".to_string(),
                    Style::default().fg(theme.border),
                )));
            } else {
                in_code_block = true;
                code_lang = raw_line.trim_start_matches('`').trim().to_string();
                let header = if code_lang.is_empty() {
                    "┌─ code ─────────────────────────────────────┐".to_string()
                } else {
                    format!(
                        "┌─ {code_lang} {}┐",
                        "─".repeat(42usize.saturating_sub(code_lang.len() + 2))
                    )
                };
                lines.push(Line::from(Span::styled(
                    header,
                    Style::default().fg(theme.border),
                )));
            }
            continue;
        }

        if in_code_block {
            lines.push(Line::from(Span::styled(
                format!("│ {raw_line}"),
                Style::default().fg(theme.code_fg),
            )));
            continue;
        }

        // ── headings ──
        if let Some(rest) = raw_line.strip_prefix("### ") {
            lines.push(Line::from(Span::styled(
                rest.to_string(),
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            )));
        } else if let Some(rest) = raw_line.strip_prefix("## ") {
            lines.push(Line::from(Span::styled(
                rest.to_string(),
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            )));
        } else if let Some(rest) = raw_line.strip_prefix("# ") {
            lines.push(Line::from(Span::styled(
                rest.to_string(),
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            )));
        }
        // ── horizontal rule ──
        else if raw_line.starts_with("---") && raw_line.chars().all(|c| c == '-') {
            lines.push(Line::from(Span::styled(
                "────────────────────────────────────────────".to_string(),
                Style::default().fg(theme.border),
            )));
        }
        // ── blockquote ──
        else if let Some(rest) = raw_line.strip_prefix("> ") {
            lines.push(Line::from(vec![
                Span::styled("▎ ", Style::default().fg(theme.border)),
                Span::styled(
                    rest.to_string(),
                    Style::default()
                        .fg(theme.dim)
                        .add_modifier(Modifier::ITALIC),
                ),
            ]));
        }
        // ── bullet list ──
        else if raw_line.starts_with("- ") || raw_line.starts_with("* ") {
            let content = &raw_line[2..];
            let spans = render_inline(content, theme);
            let mut full = vec![Span::styled(
                "  • ".to_string(),
                Style::default().fg(theme.primary),
            )];
            full.extend(spans);
            lines.push(Line::from(full));
        }
        // ── plain paragraph ──
        else {
            let spans = render_inline(raw_line, theme);
            lines.push(Line::from(spans));
        }
    }
    lines
}

/// Parse inline Markdown (`**bold**`, `*italic*`, `` `code` ``) and return
/// a `Vec<Span>`.
fn render_inline(text: &str, theme: &Theme) -> Vec<Span<'static>> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut buf = String::new();
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        // ── inline code ──
        if chars[i] == '`' {
            if !buf.is_empty() {
                spans.push(Span::raw(std::mem::take(&mut buf)));
            }
            i += 1;
            while i < len && chars[i] != '`' {
                buf.push(chars[i]);
                i += 1;
            }
            if i < len {
                i += 1; // skip closing `
            }
            spans.push(Span::styled(
                std::mem::take(&mut buf),
                Style::default().fg(theme.code_fg).bg(theme.code_bg),
            ));
            continue;
        }

        // ── bold (**…**) ──
        if i + 1 < len && chars[i] == '*' && chars[i + 1] == '*' {
            if !buf.is_empty() {
                spans.push(Span::raw(std::mem::take(&mut buf)));
            }
            i += 2;
            while i + 1 < len && !(chars[i] == '*' && chars[i + 1] == '*') {
                buf.push(chars[i]);
                i += 1;
            }
            if i + 1 < len {
                i += 2; // skip closing **
            }
            spans.push(Span::styled(
                std::mem::take(&mut buf),
                Style::default().add_modifier(Modifier::BOLD),
            ));
            continue;
        }

        // ── italic (*…*) ──
        if chars[i] == '*' {
            if !buf.is_empty() {
                spans.push(Span::raw(std::mem::take(&mut buf)));
            }
            i += 1;
            while i < len && chars[i] != '*' {
                buf.push(chars[i]);
                i += 1;
            }
            if i < len {
                i += 1;
            }
            spans.push(Span::styled(
                std::mem::take(&mut buf),
                Style::default().add_modifier(Modifier::ITALIC),
            ));
            continue;
        }

        buf.push(chars[i]);
        i += 1;
    }

    if !buf.is_empty() {
        spans.push(Span::raw(buf));
    }
    if spans.is_empty() {
        spans.push(Span::raw(String::new()));
    }
    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dark() -> Theme {
        Theme::dark()
    }

    #[test]
    fn empty_text_yields_empty_vec() {
        let lines = render_markdown("", &dark());
        assert!(lines.is_empty() || (lines.len() == 1 && lines[0].width() == 0));
    }

    #[test]
    fn h1_heading() {
        let lines = render_markdown("# Title", &dark());
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn h2_heading() {
        let lines = render_markdown("## Subtitle", &dark());
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn h3_heading() {
        let lines = render_markdown("### Section", &dark());
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn fenced_code_block() {
        let md = "```rust\nfn main() {}\n```";
        let lines = render_markdown(md, &dark());
        // header + 1 code line + footer = 3
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn bullet_list() {
        let md = "- first\n- second";
        let lines = render_markdown(md, &dark());
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn blockquote() {
        let md = "> quoted text";
        let lines = render_markdown(md, &dark());
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn horizontal_rule() {
        let md = "---";
        let lines = render_markdown(md, &dark());
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn inline_bold() {
        let spans = render_inline("hello **world**", &dark());
        assert!(spans.len() >= 2);
    }

    #[test]
    fn inline_code() {
        let spans = render_inline("use `foo::bar`", &dark());
        assert!(spans.len() >= 2);
    }

    #[test]
    fn inline_italic() {
        let spans = render_inline("this is *important*", &dark());
        assert!(spans.len() >= 2);
    }
}
