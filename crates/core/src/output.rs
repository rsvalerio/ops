//! Step line data types and display-width helpers.
//!
//! Rendering (themes, step lines, error blocks) lives in the theme crate.

use unicode_width::UnicodeWidthStr;

/// Display width of a string in terminal columns (e.g. checkmark/cross = 1, wide emoji = 2).
pub fn display_width(s: &str) -> usize {
    s.width()
}

/// Append spaces to `name` until its display width reaches `target_cols`.
///
/// DUP-3 / TASK-1235: `help.rs::render_grouped_sections`,
/// `tools_cmd.rs::run_tools_list_to`, and `theme_cmd.rs::run_theme_list_to`
/// each implemented the same `display_width` measure + manual space-pad
/// loop. Centralising here ensures any future tightening (e.g. tab
/// expansion, ZWJ-emoji handling) lands once and the three list views stay
/// column-aligned together.
///
/// Returns `name` unchanged when it already meets or exceeds `target_cols`,
/// avoiding spurious allocations on the no-pad fast path. The padding
/// counts in display columns (terminal cells) rather than `char` count, so
/// CJK / wide emoji align with ASCII names in the same column.
#[must_use]
pub fn pad_to_display_width(name: &str, target_cols: usize) -> String {
    let cols = display_width(name);
    if cols >= target_cols {
        return name.to_string();
    }
    let pad = target_cols - cols;
    // PERF-1 / TASK-1396: bulk extend replaces the per-char push loop;
    // `String::extend` over `repeat(' ').take(pad)` lowers to a single
    // reserve + memset rather than `pad` separate push branches.
    let mut out = String::with_capacity(name.len() + pad);
    out.push_str(name);
    out.extend(std::iter::repeat_n(' ', pad));
    out
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
///
/// CR/LF normalisation contract (PATTERN-1 / TASK-1094):
/// - A single trailing `\n` (optionally preceded by `\r`) is treated as a
///   line terminator and dropped, so a buffer ending in `"...\n"` does not
///   surface a phantom empty last line.
/// - A single trailing bare `\r` (no following `\n`) is also treated as a
///   terminator and dropped — this prevents stray CR bytes from rendering
///   as cursor-control characters in operator terminals.
/// - Within each emitted line, any trailing `\r` (from CRLF input) is
///   stripped and any embedded bare `\r` bytes are replaced with `\n` so
///   they cannot move the terminal cursor when the tail is rendered.
pub fn format_error_tail(stderr: &[u8], n: usize) -> String {
    format_error_tail_with_stats(stderr, n).0
}

/// Internal: returns `(rendered_tail, line_scans)` where `line_scans` is the
/// number of backwards newline searches performed. Used by structural
/// PERF-1 regression tests to assert that the byte-walk is bounded by `n`
/// (and therefore independent of total buffer size) without resorting to
/// flaky wall-clock timing assertions (TEST-15 / TASK-1029).
fn format_error_tail_with_stats(stderr: &[u8], n: usize) -> (String, usize) {
    if n == 0 || stderr.is_empty() {
        return (String::new(), 0);
    }
    // FN-1 / TASK-1405: each phase (trim, collect, decode) is now an
    // isolated helper so its byte/line invariants can be read in isolation.
    let trimmed_end = trim_trailing_terminator(stderr);
    let buf = &stderr[..trimmed_end];
    let mut ranges = TailRanges::new(n);
    let line_scans = collect_tail_ranges(buf, n, &mut ranges);
    if ranges.is_empty() {
        return (String::new(), line_scans);
    }
    let out = decode_with_cr_normalisation(buf, &ranges);
    (out, line_scans)
}

/// PATTERN-1 / TASK-1094: trim a single trailing line terminator (CRLF, LF,
/// or bare CR). Returns the new logical end index of `stderr`. A stray `\r`
/// at end-of-buffer would otherwise survive into the rendered tail and
/// render as a cursor-control byte in operator terminals.
fn trim_trailing_terminator(stderr: &[u8]) -> usize {
    let mut end = stderr.len();
    match stderr.last().copied() {
        Some(b'\n') => {
            end -= 1;
            if end > 0 && stderr[end - 1] == b'\r' {
                end -= 1;
            }
        }
        Some(b'\r') => end -= 1,
        _ => {}
    }
    end
}

/// PERF-3 / TASK-1428: collect up to `n` tail line ranges, walking backwards
/// from the end of `buf`. Returns the number of backwards line scans
/// performed (asserted by the structural PERF-1 regression test).
fn collect_tail_ranges(buf: &[u8], n: usize, ranges: &mut TailRanges) -> usize {
    let mut tail_end = buf.len();
    let mut line_scans = 0usize;
    while tail_end > 0 && ranges.len() < n {
        line_scans += 1;
        let start = buf[..tail_end]
            .iter()
            .rposition(|b| *b == b'\n')
            .map_or(0, |idx| idx + 1);
        // Strip a trailing CR so CRLF-terminated lines render cleanly.
        let mut line_end = tail_end;
        if line_end > start && buf[line_end - 1] == b'\r' {
            line_end -= 1;
        }
        ranges.push_oldest_front((start, line_end));
        tail_end = start.saturating_sub(1);
        if start == 0 {
            break;
        }
    }
    line_scans
}

/// PATTERN-1 / TASK-1094 + PERF-3 / TASK-1441: decode the collected ranges
/// into a single `String`, substituting embedded bare CR with `\n` inline so
/// progress-bar-style stderr cannot move the cursor in operator terminals.
/// The inline substitution replaces the previous
/// `from_utf8_lossy(..).replace('\r', "\n")` pair (two allocations per
/// CR-bearing line) with a single push pass into the output buffer.
fn decode_with_cr_normalisation(buf: &[u8], ranges: &TailRanges) -> String {
    let total_len: usize = ranges.iter().map(|(s, e)| e - s).sum();
    let mut out = String::with_capacity(total_len + ranges.len());
    let mut first = true;
    for &(s, e) in ranges.iter() {
        if !first {
            out.push('\n');
        }
        first = false;
        let decoded = String::from_utf8_lossy(&buf[s..e]);
        for ch in decoded.chars() {
            out.push(if ch == '\r' { '\n' } else { ch });
        }
    }
    out
}

/// PERF-3 / TASK-1428: stack-backed ring for tail line ranges. The error-tail
/// formatter is invoked per failed step with `n` config-bounded to a small
/// value (5 by default; the largest external caller passes 10). A
/// `[(usize, usize); STACK_CAP]` inline buffer covers the dominant path
/// with zero heap allocations; for the rare oversized `n` we spill into an
/// owned `Vec` only on overflow, keeping the inline buffer cold.
const TAIL_STACK_CAP: usize = 32;

struct TailRanges {
    stack: [(usize, usize); TAIL_STACK_CAP],
    stack_len: usize,
    // Spill: populated only when `n > TAIL_STACK_CAP`. Holds the oldest
    // ranges; the stack array then holds the newest `TAIL_STACK_CAP` entries.
    // For the typical small-n path this stays `Vec::new()` (no allocation).
    spill: Vec<(usize, usize)>,
}

impl TailRanges {
    fn new(n: usize) -> Self {
        let spill = if n > TAIL_STACK_CAP {
            Vec::with_capacity(n - TAIL_STACK_CAP)
        } else {
            Vec::new()
        };
        Self {
            stack: [(0, 0); TAIL_STACK_CAP],
            stack_len: 0,
            spill,
        }
    }

    fn len(&self) -> usize {
        self.stack_len + self.spill.len()
    }

    fn is_empty(&self) -> bool {
        self.stack_len == 0 && self.spill.is_empty()
    }

    /// Insert `range` as the new oldest entry. Backward walking emits the
    /// most-recent line first, so each successive push lands in front of all
    /// previously collected ranges and the iterator emits them in buffer
    /// order.
    fn push_oldest_front(&mut self, range: (usize, usize)) {
        if self.stack_len < TAIL_STACK_CAP {
            self.stack.copy_within(0..self.stack_len, 1);
            self.stack[0] = range;
            self.stack_len += 1;
        } else {
            // Stack is full: the existing oldest entry rolls over into the
            // spill (front of spill = absolute oldest).
            let overflow = self.stack[TAIL_STACK_CAP - 1];
            self.stack.copy_within(0..TAIL_STACK_CAP - 1, 1);
            self.stack[0] = range;
            self.spill.insert(0, overflow);
        }
    }

    fn iter(&self) -> impl Iterator<Item = &(usize, usize)> {
        self.spill.iter().chain(self.stack[..self.stack_len].iter())
    }
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
    ///
    /// TEST-15 / TASK-1029: the original test asserted a wall-clock budget
    /// of 50 ms which flaked on loaded CI hosts. The contract is now
    /// expressed structurally: the number of backwards line scans must be
    /// exactly `n`, independent of the input buffer size, and the decoded
    /// output length must equal the sum of the last `n` line lengths
    /// (proving we never decoded the prefix). This holds deterministically
    /// under `--release`, `--test-threads=1`, and on virtualised runners.
    #[test]
    fn format_error_tail_does_not_decode_entire_buffer() {
        let mut buf = Vec::with_capacity(4 * 1024 * 1024);
        for i in 0..200_000 {
            buf.extend_from_slice(format!("line {i}\n").as_bytes());
        }
        let buf_len = buf.len();
        let (tail, line_scans) = format_error_tail_with_stats(&buf, 5);
        // Correctness: last 5 lines decoded.
        assert!(tail.ends_with("line 199999"));
        assert!(tail.contains("line 199995"));
        // Structural PERF-1 invariant: backwards scans bounded by n,
        // independent of buffer size.
        assert_eq!(
            line_scans, 5,
            "byte-walk should perform exactly n=5 backwards scans, got {line_scans}"
        );
        // The rendered tail length must be a tiny fraction of the input —
        // proves we did not allocate a full-buffer decode. Last 5 lines
        // (`line 199995`..`line 199999` joined by `\n`) are well under 100
        // bytes; the input is ~2.2 MiB.
        assert!(
            tail.len() < 1024,
            "tail length {} should be O(n*line) not O(buffer)",
            tail.len()
        );
        assert!(
            buf_len > 1_000_000,
            "sanity: input buffer should be multi-MiB, got {buf_len}"
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

    // -- PATTERN-1 / TASK-1094: bare CR must not survive into the rendered tail --

    #[test]
    fn format_error_tail_strips_trailing_bare_cr() {
        // Buffer that ends in a bare `\r` (no following `\n`).
        let stderr = b"line1\nline2\r";
        let result = format_error_tail(stderr, 5);
        assert!(
            !result.contains('\r'),
            "rendered tail must not contain a raw CR; got {result:?}"
        );
        assert_eq!(result, "line1\nline2");
    }

    #[test]
    fn format_error_tail_strips_leading_bare_cr() {
        // The ACs require b"\rfoo" to render without a raw \r.
        let stderr = b"\rfoo";
        let result = format_error_tail(stderr, 5);
        assert!(
            !result.contains('\r'),
            "rendered tail must not contain a raw CR; got {result:?}"
        );
        assert!(result.contains("foo"));
    }

    #[test]
    fn format_error_tail_normalises_embedded_bare_cr() {
        // Bare CR inside a line (e.g. progress-bar updates) would otherwise
        // move the cursor to column 0 in operator terminals.
        let stderr = b"first\nbar\rbaz\n";
        let result = format_error_tail(stderr, 5);
        assert!(
            !result.contains('\r'),
            "rendered tail must not contain a raw CR; got {result:?}"
        );
    }

    #[test]
    fn format_error_tail_only_bare_cr_buffer() {
        // A corrupt single-byte input of just `\r` should render as empty,
        // not as a literal CR.
        let stderr = b"\r";
        let result = format_error_tail(stderr, 5);
        assert!(
            !result.contains('\r'),
            "rendered tail must not contain a raw CR; got {result:?}"
        );
        assert!(result.is_empty());
    }
}
