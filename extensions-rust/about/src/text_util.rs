//! Text utilities: formatting, padding, truncation, and wrapping.

use ops_core::output::display_width;
pub(crate) use ops_core::text::format_number;

pub(crate) fn get_terminal_width() -> usize {
    std::env::var("COLUMNS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(120)
}

pub(crate) fn char_display_width(c: char) -> usize {
    unicode_width::UnicodeWidthChar::width(c).unwrap_or(0)
}

pub(crate) fn pad_to_width_plain(s: &str, width: usize) -> String {
    let current_width = s.chars().map(char_display_width).sum::<usize>();
    if current_width >= width {
        s.to_string()
    } else {
        format!("{}{}", s, " ".repeat(width - current_width))
    }
}

pub(crate) fn truncate_to_width(s: &str, max_width: usize) -> String {
    let mut result = String::new();
    let mut width = 0;

    for c in s.chars() {
        let c_width = char_display_width(c);
        if width + c_width > max_width.saturating_sub(1) {
            result.push('\u{2026}');
            break;
        }
        result.push(c);
        width += c_width;
    }

    result
}

pub(crate) fn wrap_text(text: &str, max_width: usize, max_lines: usize) -> Vec<String> {
    if text.is_empty() || max_lines == 0 {
        return vec![];
    }

    let words: Vec<&str> = text.split_whitespace().collect();
    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in words {
        let word_width = display_width(word);
        let current_width = display_width(&current_line);

        if current_line.is_empty() {
            current_line = word.to_string();
        } else if current_width + 1 + word_width <= max_width {
            current_line = format!("{} {}", current_line, word);
        } else {
            lines.push(current_line);
            current_line = word.to_string();

            if lines.len() >= max_lines {
                break;
            }
        }
    }

    if !current_line.is_empty() && lines.len() < max_lines {
        lines.push(current_line);
    }

    lines.truncate(max_lines);

    if let Some(last) = lines.last_mut() {
        if display_width(last) > max_width.saturating_sub(1) {
            *last = truncate_to_width(last, max_width);
        }
    }

    lines
}

pub(crate) fn tty_style(text: &str, styler: fn(&str) -> String, is_tty: bool) -> String {
    if is_tty {
        styler(text)
    } else {
        text.to_string()
    }
}

pub(crate) fn pad_header(left: &str, right: &str) -> String {
    use super::cards::CardLayoutConfig;
    let left_display = display_width(left);
    let right_display = display_width(right);
    let target_content_width = CardLayoutConfig::BOX_WIDTH - 2;

    let padding = target_content_width.saturating_sub(left_display + right_display + 1);
    format!("{}{}{} ", left, " ".repeat(padding), right)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::CardLayoutConfig;
    use ops_core::output::display_width;

    #[test]
    fn truncate_to_width_short_string() {
        assert_eq!(truncate_to_width("hello", 10), "hello");
    }

    #[test]
    fn truncate_to_width_exact_fit() {
        assert_eq!(truncate_to_width("hello", 5), "hell\u{2026}");
    }

    #[test]
    fn truncate_to_width_needs_truncation() {
        assert_eq!(truncate_to_width("hello world", 6), "hello\u{2026}");
    }

    #[test]
    fn truncate_to_width_very_short_max() {
        let result = truncate_to_width("hello", 1);
        assert_eq!(result, "\u{2026}");
    }

    #[test]
    fn truncate_to_width_empty() {
        assert_eq!(truncate_to_width("", 10), "");
    }

    #[test]
    fn wrap_text_single_line() {
        let result = wrap_text("hello world", 20, 2);
        assert_eq!(result, vec!["hello world"]);
    }

    #[test]
    fn wrap_text_multiple_lines() {
        let result = wrap_text("one two three four five", 10, 3);
        assert!(result.len() <= 3);
        for line in &result {
            assert!(display_width(line) <= 10);
        }
    }

    #[test]
    fn wrap_text_respects_max_lines() {
        let result = wrap_text("one two three four five six seven eight", 5, 2);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn wrap_text_empty() {
        let result = wrap_text("", 10, 2);
        assert!(result.is_empty());
    }

    #[test]
    fn wrap_text_max_lines_zero() {
        let result = wrap_text("hello world", 20, 0);
        assert!(result.is_empty());
    }

    #[test]
    fn wrap_text_long_word_exceeds_width() {
        let result = wrap_text("superlongword short", 5, 3);
        assert!(!result.is_empty());
        assert!(result[0].contains("superlongword") || result[0].contains("super"));
    }

    #[test]
    fn pad_to_width_adds_padding() {
        let result = pad_to_width_plain("hi", 5);
        assert_eq!(result.len(), 5);
    }

    #[test]
    fn pad_to_width_already_wide() {
        let result = pad_to_width_plain("hello", 3);
        assert_eq!(result, "hello");
    }

    #[test]
    fn char_display_width_ascii() {
        assert_eq!(char_display_width('a'), 1);
        assert_eq!(char_display_width(' '), 1);
    }

    #[test]
    fn char_display_width_wide() {
        assert_eq!(char_display_width('\u{6f22}'), 2);
    }

    #[test]
    fn char_display_width_zero_width() {
        assert_eq!(char_display_width('\u{200D}'), 0);
    }

    #[test]
    fn tty_style_applies_when_tty() {
        let styled = tty_style("hello", ops_core::style::cyan, true);
        assert!(styled.contains("hello"));
        assert!(styled.contains("\x1b["));
    }

    #[test]
    fn tty_style_passthrough_when_not_tty() {
        let result = tty_style("hello", ops_core::style::cyan, false);
        assert_eq!(result, "hello");
    }

    #[test]
    fn get_terminal_width_default() {
        let saved = std::env::var("COLUMNS").ok();
        std::env::remove_var("COLUMNS");
        let width = get_terminal_width();
        assert_eq!(width, 120);
        if let Some(v) = saved {
            std::env::set_var("COLUMNS", v);
        }
    }

    #[test]
    fn pad_header_balances_left_and_right() {
        let result = pad_header("Left", "Right");
        assert!(result.starts_with("Left"));
        assert!(result.ends_with("Right "));
        assert!(result.len() <= CardLayoutConfig::BOX_WIDTH);
    }

    #[test]
    fn pad_header_long_strings() {
        let left = "A".repeat(60);
        let right = "B".repeat(60);
        let result = pad_header(&left, &right);
        assert!(result.contains(&left));
        assert!(result.contains(&right));
    }
}
