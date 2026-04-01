//! Simple Markdown stripping and ANSI terminal rendering.

/// Strip common Markdown syntax, returning plain text.
#[must_use]
pub fn strip_markdown(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_code_block = false;

    for line in text.lines() {
        let trimmed = line.trim();

        // Toggle fenced code blocks
        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }
        if in_code_block {
            out.push_str(line);
            out.push('\n');
            continue;
        }
        // Strip headings
        let line = if trimmed.starts_with('#') {
            trimmed.trim_start_matches('#').trim()
        } else {
            trimmed
        };
        // Strip bold / italic markers
        let line = line.replace("**", "").replace("__", "");
        let line = strip_single_emphasis(&line);
        // Strip inline code backticks
        let line = line.replace('`', "");
        // Strip links [text](url) → text
        let line = strip_links(&line);
        // Strip images ![alt](url) → alt
        let line = strip_images(&line);
        // Strip horizontal rules
        if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            continue;
        }

        out.push_str(&line);
        out.push('\n');
    }
    out.trim_end().to_owned()
}

/// Render Markdown to terminal output with ANSI codes.
#[must_use]
pub fn render_markdown_to_terminal(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 256);
    let mut in_code_block = false;

    for line in text.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            if in_code_block {
                out.push_str("  \x1b[2m"); // dim
            } else {
                out.push_str("\x1b[0m");
            }
            out.push('\n');
            continue;
        }

        if in_code_block {
            out.push_str("    ");
            out.push_str(line);
            out.push('\n');
            continue;
        }

        // Headings → bold
        if trimmed.starts_with('#') {
            let heading = trimmed.trim_start_matches('#').trim();
            out.push_str(&format!("\x1b[1m{heading}\x1b[0m\n"));
            continue;
        }

        // Bold markers → ANSI bold
        let line = line.replace("**", "\x1b[1m");
        // Inline code → dim
        let line = line.replace('`', "\x1b[2m");

        out.push_str(&line);
        out.push('\n');
    }

    if in_code_block {
        out.push_str("\x1b[0m\n");
    }
    out
}

/// Strip single `*` or `_` emphasis (non-greedy).
fn strip_single_emphasis(s: &str) -> String {
    let mut result = s.to_owned();
    for marker in &["*", "_"] {
        while let Some(start) = result.find(marker) {
            if let Some(end) = result[start + 1..].find(marker) {
                let inner = &result[start + 1..start + 1 + end];
                result = format!("{}{}{}", &result[..start], inner, &result[start + 1 + end + 1..]);
            } else {
                break;
            }
        }
    }
    result
}

/// Strip `[text](url)` → `text`.
fn strip_links(s: &str) -> String {
    let mut result = s.to_owned();
    while let Some(open) = result.find('[') {
        if let Some(close) = result[open..].find("](") {
            let close_abs = open + close;
            if let Some(paren_close) = result[close_abs + 2..].find(')') {
                let text = &result[open + 1..close_abs];
                let text = text.to_owned();
                result = format!(
                    "{}{}{}",
                    &result[..open],
                    text,
                    &result[close_abs + 2 + paren_close + 1..]
                );
                continue;
            }
        }
        break;
    }
    result
}

/// Strip `![alt](url)` → `alt`.
fn strip_images(s: &str) -> String {
    let mut result = s.to_owned();
    while let Some(pos) = result.find("![") {
        if let Some(close) = result[pos + 2..].find("](") {
            let close_abs = pos + 2 + close;
            if let Some(paren_close) = result[close_abs + 2..].find(')') {
                let alt = &result[pos + 2..close_abs];
                let alt = alt.to_owned();
                result = format!(
                    "{}{}{}",
                    &result[..pos],
                    alt,
                    &result[close_abs + 2 + paren_close + 1..]
                );
                continue;
            }
        }
        break;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_markdown_headings() {
        let md = "# Title\n## Subtitle\nPlain text";
        let plain = strip_markdown(md);
        assert!(plain.contains("Title"));
        assert!(plain.contains("Plain text"));
        assert!(!plain.contains('#'));
    }

    #[test]
    fn test_render_markdown_bold() {
        let md = "**bold text**";
        let rendered = render_markdown_to_terminal(md);
        assert!(rendered.contains("\x1b[1m"));
        assert!(rendered.contains("bold text"));
    }
}
