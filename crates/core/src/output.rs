//! Step line data types and display-width helpers.
//!
//! Rendering (themes, step lines, error blocks) lives in the theme crate.

use unicode_width::UnicodeWidthStr;

/// Display width of a string in terminal columns (e.g. checkmark/cross = 1, wide emoji = 2).
pub fn display_width(s: &str) -> usize {
    s.width()
}

/// Live terminal width in columns reported by the OS (`ioctl(TIOCGWINSZ)` or
/// the Windows console handle), or `None` when no terminal is attached.
///
/// ARCH-2 / TASK-0667: prefer this over reading the `COLUMNS` environment
/// variable, which is set on demand by interactive shells but is unset under
/// most non-interactive parents (CI, `cargo run`, IDE terminals before the
/// first resize). Callers should fall back to `COLUMNS` only when stdout is
/// not a TTY.
#[must_use]
pub fn detect_terminal_width() -> Option<usize> {
    terminal_size::terminal_size().map(|(w, _)| usize::from(w.0))
}

/// Return the last `n` lines from a slice, or all lines if fewer than `n`.
pub fn tail_lines<T>(lines: &[T], n: usize) -> &[T] {
    let start = lines.len().saturating_sub(n);
    &lines[start..]
}

/// Format the last `n` lines of stderr for error display.
///
/// PERF-1 (TASK-0733): scans the buffer from the end via byte-wise newline
/// search, decoding only the tail segments instead of decoding the entire
/// `stderr` (which can be megabytes under a failed `cargo test`). Memory and
/// CPU cost are O(n * average-line-length) regardless of input size.
pub fn format_error_tail(stderr: &[u8], n: usize) -> String {
    if n == 0 || stderr.is_empty() {
        return String::new();
    }

    // Trim a single trailing newline so a buffer ending in "...\n" does not
    // surface a phantom empty last line (matches the prior `str::lines()`
    // semantics which suppress the trailing empty segment).
    let mut end = stderr.len();
    if stderr.last() == Some(&b'\n') {
        end -= 1;
        if stderr.get(end.wrapping_sub(1)).copied() == Some(b'\r') {
            end -= 1;
        }
    }

    // Walk backwards collecting up to `n` line ranges. Each range stops at
    // the byte after the preceding `\n` (or 0 for the first line).
    let buf = &stderr[..end];
    let mut ranges: std::collections::VecDeque<(usize, usize)> =
        std::collections::VecDeque::with_capacity(n);
    let mut tail_end = buf.len();
    while tail_end > 0 && ranges.len() < n {
        let start = match buf[..tail_end].iter().rposition(|b| *b == b'\n') {
            Some(idx) => idx + 1,
            None => 0,
        };
        // Strip a trailing CR so CRLF-terminated lines render cleanly.
        let mut line_end = tail_end;
        if buf.get(line_end.wrapping_sub(1)).copied() == Some(b'\r') {
            line_end -= 1;
        }
        ranges.push_front((start, line_end));
        tail_end = start.saturating_sub(1);
        if start == 0 {
            break;
        }
    }

    if ranges.is_empty() {
        return String::new();
    }

    // Decode only the tail segments, joining without an intermediate Vec.
    let mut out =
        String::with_capacity(ranges.iter().map(|(s, e)| e - s).sum::<usize>() + ranges.len());
    let mut first = true;
    for (s, e) in ranges {
        if !first {
            out.push('\n');
        }
        first = false;
        out.push_str(&String::from_utf8_lossy(&buf[s..e]));
    }
    out
}

/// Logical status of a step for step-line rendering.
///
/// API-9 / TASK-0454: marked `#[non_exhaustive]` so adding a new variant
/// (e.g. `Cancelled`) is not a breaking change for downstream consumers
/// (themes, runner, extensions). Out-of-crate `match` sites must include a
/// wildcard arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum StepStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    Skipped,
}

/// Data for one step line: status, command label, and optional elapsed time.
///
/// API-9 / TASK-0454: marked `#[non_exhaustive]` so adding fields is not a
/// breaking change for downstream consumers. Construct via [`StepLine::new`]
/// rather than struct-literal syntax.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct StepLine {
    pub status: StepStatus,
    pub label: String,
    pub elapsed: Option<f64>,
}

impl StepLine {
    /// Constructor used by themes / runner / tests in place of struct-literal
    /// initialization, which is forbidden under `#[non_exhaustive]`.
    #[must_use]
    pub fn new(status: StepStatus, label: impl Into<String>, elapsed: Option<f64>) -> Self {
        Self {
            status,
            label: label.into(),
            elapsed,
        }
    }
}

/// Error details shown inline below a failed step line.
///
/// Contains the exit message and an optional tail of stderr output.
///
/// API-9 / TASK-0454: marked `#[non_exhaustive]` so adding fields is not a
/// breaking change. Construct via [`ErrorDetail::new`].
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ErrorDetail {
    /// Primary error message (e.g. "exit status: 101").
    pub message: String,
    /// Last N lines of stderr captured from the failed command.
    pub stderr_tail: Vec<String>,
}

impl ErrorDetail {
    /// Constructor used in place of struct-literal initialization.
    #[must_use]
    pub fn new(message: impl Into<String>, stderr_tail: Vec<String>) -> Self {
        Self {
            message: message.into(),
            stderr_tail,
        }
    }
}

/// All possible step statuses, used by themes to compute max icon width.
///
/// This constant contains all 5 variants of [`StepStatus`] in a fixed array.
/// Themes iterate over this to find the widest icon and pad narrower icons
/// for column alignment in step-line rendering.
pub const ALL_STATUSES: [StepStatus; 5] = [
    StepStatus::Pending,
    StepStatus::Running,
    StepStatus::Succeeded,
    StepStatus::Failed,
    StepStatus::Skipped,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_width_ascii() {
        assert_eq!(display_width("hello"), 5);
    }

    #[test]
    fn display_width_emoji() {
        assert_eq!(display_width("\u{2705}"), 2);
    }

    #[test]
    fn tail_lines_returns_last_n() {
        let lines = vec!["a", "b", "c", "d", "e"];
        assert_eq!(tail_lines(&lines, 3), &["c", "d", "e"]);
    }

    #[test]
    fn tail_lines_returns_all_when_fewer_than_n() {
        let lines = vec!["a", "b"];
        assert_eq!(tail_lines(&lines, 5), &["a", "b"]);
    }

    #[test]
    fn tail_lines_empty_returns_empty() {
        let lines: Vec<&str> = vec![];
        assert!(tail_lines(&lines, 3).is_empty());
    }

    #[test]
    fn step_line_construction() {
        let line = StepLine {
            status: StepStatus::Succeeded,
            label: "cargo build".to_string(),
            elapsed: Some(1.5),
        };
        assert_eq!(line.status, StepStatus::Succeeded);
        assert_eq!(line.label, "cargo build");
        assert_eq!(line.elapsed, Some(1.5));
    }

    #[test]
    fn error_detail_construction() {
        let detail = ErrorDetail {
            message: "exit status: 1".to_string(),
            stderr_tail: vec!["error line 1".to_string(), "error line 2".to_string()],
        };
        assert_eq!(detail.message, "exit status: 1");
        assert_eq!(detail.stderr_tail.len(), 2);
    }

    // -- TQ-011: Edge cases for display_width --

    #[test]
    fn display_width_empty_string() {
        assert_eq!(display_width(""), 0);
    }

    #[test]
    fn display_width_unicode_combining() {
        assert!(display_width("e\u{0301}") >= 1);
    }

    #[test]
    fn display_width_very_long_string() {
        let long = "a".repeat(1000);
        assert_eq!(display_width(&long), 1000);
    }

    #[test]
    fn display_width_zero_width_joiner() {
        let family = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}";
        assert!(
            display_width(family) >= 2,
            "family emoji should have display width"
        );
    }

    #[test]
    fn display_width_control_chars() {
        assert!(
            display_width("\x00\x01\x02") > 0,
            "control chars are counted by unicode-width"
        );
    }

    #[test]
    fn display_width_mixed_ascii_unicode() {
        assert_eq!(display_width("abc\u{2713}def"), 7);
    }

    // -- Tests for format_error_tail --

    #[test]
    fn format_error_tail_returns_last_n_lines() {
        let stderr = b"line1\nline2\nline3\nline4\nline5";
        let result = format_error_tail(stderr, 3);
        assert_eq!(result, "line3\nline4\nline5");
    }

    #[test]
    fn format_error_tail_handles_fewer_lines() {
        let stderr = b"line1\nline2";
        let result = format_error_tail(stderr, 5);
        assert_eq!(result, "line1\nline2");
    }

    #[test]
    fn format_error_tail_handles_empty() {
        let result = format_error_tail(b"", 5);
        assert!(result.is_empty());
    }

    #[test]
    fn format_error_tail_handles_invalid_utf8() {
        let stderr = b"line1\n\xff\xfe\nline3";
        let result = format_error_tail(stderr, 5);
        assert!(result.contains("line1"));
        assert!(result.contains("line3"));
    }

    /// PERF-1 (TASK-0733): a multi-MiB stderr buffer must not be fully
    /// decoded just to surface the last 5 lines. Pre-fix this allocated the
    /// full buffer via `String::from_utf8_lossy(stderr).into_owned()` —
    /// noticeable on failed builds and easy to regress under refactor.
    #[test]
    fn format_error_tail_does_not_decode_entire_buffer() {
        let mut buf = Vec::with_capacity(4 * 1024 * 1024);
        for i in 0..200_000 {
            buf.extend_from_slice(format!("line {i}\n").as_bytes());
        }
        let start = std::time::Instant::now();
        let tail = format_error_tail(&buf, 5);
        let elapsed = start.elapsed();
        assert!(tail.ends_with("line 199999"));
        assert!(tail.contains("line 199995"));
        assert!(
            elapsed < std::time::Duration::from_millis(50),
            "tail extraction should not scale with buffer size; took {elapsed:?}"
        );
    }

    #[test]
    fn format_error_tail_strips_trailing_newline() {
        let stderr = b"line1\nline2\n";
        let result = format_error_tail(stderr, 5);
        assert_eq!(result, "line1\nline2");
    }

    #[test]
    fn format_error_tail_handles_crlf() {
        let stderr = b"line1\r\nline2\r\n";
        let result = format_error_tail(stderr, 5);
        assert_eq!(result, "line1\nline2");
    }

    #[test]
    fn format_error_tail_n_zero_returns_empty() {
        let stderr = b"line1\nline2";
        assert!(format_error_tail(stderr, 0).is_empty());
    }
}
