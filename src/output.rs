//! Step line data types and display-width helpers.
//!
//! Rendering (themes, step lines, error blocks) lives in the [`crate::theme`] module.

use unicode_width::UnicodeWidthStr;

/// Display width of a string in terminal columns (e.g. ✓/✗ = 1, ✅ = 2).
pub fn display_width(s: &str) -> usize {
    s.width()
}

/// Return the last `n` lines from a slice, or all lines if fewer than `n`.
pub fn tail_lines<T>(lines: &[T], n: usize) -> &[T] {
    let start = lines.len().saturating_sub(n);
    &lines[start..]
}

/// Format the last `n` lines of stderr for error display.
/// Converts raw bytes to string (lossy), extracts lines, and joins with newlines.
#[cfg(feature = "stack-rust")]
pub fn format_error_tail(stderr: &[u8], n: usize) -> String {
    let stderr_str = String::from_utf8_lossy(stderr);
    let lines: Vec<&str> = stderr_str.lines().collect();
    tail_lines(&lines, n).join("\n")
}

/// Logical status of a step for step-line rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    Skipped,
}

/// Data for one step line: status, command label, and optional elapsed time.
#[derive(Debug, Clone)]
pub struct StepLine {
    pub status: StepStatus,
    pub label: String,
    pub elapsed: Option<f64>,
}

/// Error details shown inline below a failed step line.
///
/// Contains the exit message and an optional tail of stderr output.
/// Rendered by the theme's [`crate::theme::StepLineTheme::render_error_detail`].
#[derive(Debug, Clone)]
pub struct ErrorDetail {
    /// Primary error message (e.g. "exit status: 101").
    pub message: String,
    /// Last N lines of stderr captured from the failed command.
    pub stderr_tail: Vec<String>,
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
        assert_eq!(display_width("✅"), 2);
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

    // -- TQ-010: Tests for StepLine, ErrorDetail, StepStatus --

    #[test]
    fn all_statuses_covered_in_match() {
        fn assert_exhaustive(status: StepStatus) -> bool {
            match status {
                StepStatus::Pending => true,
                StepStatus::Running => true,
                StepStatus::Succeeded => true,
                StepStatus::Failed => true,
                StepStatus::Skipped => true,
            }
        }
        for status in ALL_STATUSES {
            assert!(assert_exhaustive(status));
        }
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

    #[test]
    fn all_statuses_has_five_variants() {
        assert_eq!(ALL_STATUSES.len(), 5);
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
        let family = "👨‍👩‍👧";
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
        assert_eq!(display_width("abc✓def"), 7);
    }

    // -- Tests for format_error_tail --

    #[cfg(feature = "stack-rust")]
    #[test]
    fn format_error_tail_returns_last_n_lines() {
        let stderr = b"line1\nline2\nline3\nline4\nline5";
        let result = format_error_tail(stderr, 3);
        assert_eq!(result, "line3\nline4\nline5");
    }

    #[cfg(feature = "stack-rust")]
    #[test]
    fn format_error_tail_handles_fewer_lines() {
        let stderr = b"line1\nline2";
        let result = format_error_tail(stderr, 5);
        assert_eq!(result, "line1\nline2");
    }

    #[cfg(feature = "stack-rust")]
    #[test]
    fn format_error_tail_handles_empty() {
        let result = format_error_tail(b"", 5);
        assert!(result.is_empty());
    }

    #[cfg(feature = "stack-rust")]
    #[test]
    fn format_error_tail_handles_invalid_utf8() {
        let stderr = b"line1\n\xff\xfe\nline3";
        let result = format_error_tail(stderr, 5);
        assert!(result.contains("line1"));
        assert!(result.contains("line3"));
    }
}
