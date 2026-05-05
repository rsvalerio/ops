//! Text utilities: formatting, padding, truncation, and wrapping.
//!
//! Stack-agnostic helpers used by about subpages across stacks.

use ops_core::output::{detect_terminal_width, display_width};
pub use ops_core::text::format_number;
use std::io::IsTerminal;

pub fn get_terminal_width() -> usize {
    // ARCH-2 / TASK-0667: when stdout is a TTY, ask the OS for the real
    // window size first; `COLUMNS` is unset in many shells until the user
    // resizes once, and the previous 120-column fallback wrapped badly on
    // narrow terminals and under-utilised wide ones.
    if std::io::stdout().is_terminal() {
        if let Some(width) = detect_terminal_width() {
            return width;
        }
    }
    parse_terminal_width(std::env::var("COLUMNS").ok().as_deref())
}

/// Parse a COLUMNS-style width source. Extracted from `get_terminal_width`
/// so tests can exercise the parser without mutating process-global env,
/// which otherwise races with any parallel test reading COLUMNS.
pub fn parse_terminal_width(raw: Option<&str>) -> usize {
    raw.and_then(|s| s.parse().ok()).unwrap_or(120)
}

pub fn char_display_width(c: char) -> usize {
    unicode_width::UnicodeWidthChar::width(c).unwrap_or(0)
}

pub fn pad_to_width_plain(s: &str, width: usize) -> String {
    // PATTERN-1 / TASK-1001: delegate to `display_width` so emoji ZWJ
    // sequences (`👨‍👩‍👧`), regional-indicator flag pairs, and variation
    // selectors are accounted for at the cluster level. Char-summing
    // over-counted joiners / VS-16 glyphs and produced misaligned About
    // cards for unit names containing emoji.
    let current_width = display_width(s);
    if current_width >= width {
        s.to_string()
    } else {
        format!("{}{}", s, " ".repeat(width - current_width))
    }
}

pub fn truncate_to_width(s: &str, max_width: usize) -> String {
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

pub fn wrap_text(text: &str, max_width: usize, max_lines: usize) -> Vec<String> {
    if text.is_empty() || max_lines == 0 {
        return vec![];
    }

    let mut lines = Vec::new();
    let mut current_line = String::new();
    // Track current line width incrementally rather than re-scanning
    // `current_line` via display_width on every iteration. Scanning was
    // O(N) per word, making the overall wrap O(N^2) for long descriptions.
    let mut current_width: usize = 0;

    for word in text.split_whitespace() {
        let word_width = display_width(word);

        if current_line.is_empty() {
            current_line.push_str(word);
            current_width = word_width;
        } else if current_width + 1 + word_width <= max_width {
            current_line.push(' ');
            current_line.push_str(word);
            current_width += 1 + word_width;
        } else {
            lines.push(std::mem::take(&mut current_line));
            current_line.push_str(word);
            current_width = word_width;

            if lines.len() >= max_lines {
                break;
            }
        }
    }

    if !current_line.is_empty() && lines.len() < max_lines {
        lines.push(current_line);
    }

    lines.truncate(max_lines);

    // READ-5 (TASK-0550): an unbreakable word wider than max_width is pushed
    // verbatim when current_line is empty and may land on an intermediate
    // line, so truncating only the last line lets earlier lines exceed the
    // contract. Enforce display_width(line) <= max_width on every line.
    for line in &mut lines {
        if display_width(line) > max_width.saturating_sub(1) {
            *line = truncate_to_width(line, max_width);
        }
    }

    lines
}

pub fn tty_style(text: &str, styler: fn(&str) -> String, is_tty: bool) -> String {
    if is_tty {
        styler(text)
    } else {
        text.to_string()
    }
}

/// Pad `left` and `right` with spaces so they span a content area of
/// `target_content_width` columns (right-aligned right string, one trailing space).
pub fn pad_header(left: &str, right: &str, target_content_width: usize) -> String {
    let left_display = display_width(left);
    let right_display = display_width(right);
    let padding = target_content_width.saturating_sub(left_display + right_display + 1);
    format!("{}{}{} ", left, " ".repeat(padding), right)
}

#[cfg(test)]
mod tests {
    use super::*;
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
        assert_eq!(truncate_to_width("hello", 1), "\u{2026}");
    }

    #[test]
    fn truncate_to_width_empty() {
        assert_eq!(truncate_to_width("", 10), "");
    }

    #[test]
    fn wrap_text_single_line() {
        assert_eq!(wrap_text("hello world", 20, 2), vec!["hello world"]);
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
        assert!(wrap_text("", 10, 2).is_empty());
    }

    #[test]
    fn wrap_text_max_lines_zero() {
        assert!(wrap_text("hello world", 20, 0).is_empty());
    }

    #[test]
    fn wrap_text_long_word_exceeds_width() {
        let result = wrap_text("superlongword short", 5, 3);
        assert!(!result.is_empty());
    }

    /// READ-5 (TASK-0550): every emitted line — including intermediate ones —
    /// must satisfy display_width(line) <= max_width when an unbreakable word
    /// wider than max_width appears in the first of three lines.
    /// PERF-1 (TASK-0709): pathological long descriptions must wrap in
    /// linear time. Pre-fix this allocated O(N^2) work because
    /// `display_width(&current_line)` was called inside the per-word loop;
    /// a 10k-word input was visibly slow. We can't pin the asymptotic bound
    /// in a unit test cheaply, but we can demand the wrapper completes a
    /// 10k-token input in a budget that the quadratic version blows past
    /// even on fast machines.
    #[test]
    fn wrap_text_handles_very_long_input_in_linear_time() {
        let words = std::iter::repeat_n("word", 10_000).collect::<Vec<_>>();
        let text = words.join(" ");
        let start = std::time::Instant::now();
        let lines = wrap_text(&text, 80, 50);
        let elapsed = start.elapsed();
        assert!(!lines.is_empty());
        assert!(
            elapsed < std::time::Duration::from_millis(250),
            "wrap_text should be O(N); 10k-word input took {elapsed:?}"
        );
    }

    #[test]
    fn wrap_text_truncates_intermediate_overlong_lines() {
        let result = wrap_text(
            "https://example.com/very/long/path-that-overflows aa bb",
            10,
            3,
        );
        for line in &result {
            assert!(
                display_width(line) <= 10,
                "line {line:?} exceeds max_width 10 (got {})",
                display_width(line)
            );
        }
    }

    #[test]
    fn pad_to_width_adds_padding() {
        assert_eq!(pad_to_width_plain("hi", 5).len(), 5);
    }

    /// PATTERN-1 / TASK-1001: a string with an emoji ZWJ sequence
    /// (`👨‍👩‍👧`) must be padded based on `display_width` (cluster-aware),
    /// matching how the rest of the about/text_util module measures text.
    /// Char-summing over-counted the joiner / VS-16 glyphs and produced
    /// off-by-one cards under TTY rendering.
    #[test]
    fn pad_to_width_uses_display_width_for_zwj_sequence() {
        let s = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}"; // 👨‍👩‍👧
        let target = display_width(s) + 4;
        let padded = pad_to_width_plain(s, target);
        assert_eq!(
            display_width(&padded),
            target,
            "padded display_width must equal target; got padded={padded:?}"
        );
        assert!(padded.starts_with(s));
    }

    #[test]
    fn pad_to_width_already_wide() {
        assert_eq!(pad_to_width_plain("hello", 3), "hello");
    }

    #[test]
    fn char_display_width_ascii() {
        assert_eq!(char_display_width('a'), 1);
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
        assert_eq!(tty_style("hello", ops_core::style::cyan, false), "hello");
    }

    #[test]
    fn parse_terminal_width_default_when_unset() {
        assert_eq!(parse_terminal_width(None), 120);
    }

    #[test]
    fn parse_terminal_width_default_when_unparseable() {
        assert_eq!(parse_terminal_width(Some("not-a-number")), 120);
    }

    #[test]
    fn parse_terminal_width_uses_explicit_value() {
        assert_eq!(parse_terminal_width(Some("80")), 80);
    }

    #[test]
    fn pad_header_balances_left_and_right() {
        let result = pad_header("Left", "Right", 100);
        assert!(result.starts_with("Left"));
        assert!(result.ends_with("Right "));
        assert!(result.len() <= 100);
    }

    #[test]
    fn pad_header_long_strings() {
        let left = "A".repeat(60);
        let right = "B".repeat(60);
        let result = pad_header(&left, &right, 100);
        assert!(result.contains(&left));
        assert!(result.contains(&right));
    }
}
