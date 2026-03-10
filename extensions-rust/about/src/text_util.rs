//! Text utilities: formatting, padding, truncation, and wrapping.

use cargo_ops_core::output::display_width;

pub(crate) fn format_number(n: i64) -> String {
    if n < 0 {
        return format!("-{}", format_number(-n));
    }
    let s = n.to_string();
    let mut result = String::new();
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(ch);
    }
    result.chars().rev().collect()
}

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
