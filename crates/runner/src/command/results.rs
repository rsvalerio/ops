//! Result types for command execution.

use ops_core::config::CommandId;
use std::sync::OnceLock;
use std::time::Duration;

/// Result of running a single command.
#[derive(Debug, Clone)]
#[must_use = "StepResult carries success/failure, stderr, and timing; discarding it hides command failures"]
#[non_exhaustive]
pub struct StepResult {
    pub id: CommandId,
    pub success: bool,
    pub duration: Duration,
    pub stdout: String,
    pub stderr: String,
    pub message: Option<String>,
}

impl StepResult {
    /// DUP-003: Private helper to construct base StepResult with common defaults.
    fn new(id: impl Into<CommandId>, success: bool, duration: Duration) -> Self {
        Self {
            id: id.into(),
            success,
            duration,
            stdout: String::new(),
            stderr: String::new(),
            message: None,
        }
    }

    /// Construct a successful `StepResult` with captured stdout.
    ///
    /// Out-of-crate callers (CLI tests, downstream tools) cannot use struct
    /// literals once the type is `#[non_exhaustive]`; this provides a stable
    /// path for the success-with-output shape.
    pub fn success_with_stdout(
        id: impl Into<CommandId>,
        duration: Duration,
        stdout: String,
    ) -> Self {
        Self {
            stdout,
            ..Self::new(id, true, duration)
        }
    }

    /// Construct a failure result for an IO/timeout error (no captured output).
    pub fn failure(id: impl Into<CommandId>, duration: Duration, message: String) -> Self {
        Self {
            message: Some(message),
            ..Self::new(id, false, duration)
        }
    }

    /// ERR-1 / TASK-0408: a cancellation happens when a sibling task
    /// triggered `fail_fast` (parallel) or the abort flag was already set
    /// before this task started (exec_standalone). Encoded as
    /// `success: false` so plan-success aggregation
    /// (`results.iter().all(|r| r.success)`) yields a non-zero exit code
    /// even in the (currently impossible but architecturally fragile)
    /// scenario where the originating failure is filtered or buffered out
    /// of the same result vector. The previous `StepResult::skipped`
    /// constructor used `success: true` for this case, overloading the
    /// "this step succeeded" contract with "this step never ran because we
    /// cancelled it" — distinguishable only by caller convention.
    ///
    /// Display still renders the row using the `StepSkipped` event the
    /// executor emitted; cancelled and skipped look identical on screen by
    /// design (intent is conveyed by the surrounding failure context, not
    /// the row itself).
    pub fn cancelled(id: impl Into<CommandId>) -> Self {
        Self::new(id, false, Duration::ZERO)
    }

    #[cfg(test)]
    pub fn success(id: &str, duration: Duration) -> Self {
        Self::new(id, true, duration)
    }
}

/// Captured output from a command execution.
/// Note: Invalid UTF-8 bytes are replaced with U+FFFD replacement character.
/// This is acceptable for CLI display but loses original byte sequences.
///
/// EFF-004: We use `into_owned()` on `Cow<str>` to convert to `String`. This is
/// necessary because we need owned strings for the `StepResult` struct which may
/// outlive the original `std::process::Output`. For most CLI outputs (which are
/// typically small), this overhead is negligible. If processing very large outputs
/// (e.g., >1MB), consider streaming output directly instead of buffering.
#[derive(Debug)]
#[non_exhaustive]
pub struct CommandOutput {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub status_message: String,
}

/// PERF-1 / TASK-0515: per-stream byte cap for captured stdout/stderr.
///
/// A pathological build (e.g. a runaway logger or a `cargo test` flooding
/// stderr) used to balloon the per-step `String` to hundreds of MB. Now we
/// keep the head of the stream up to this cap and append a single marker
/// line so the consumer (UI tail / TAP file) sees that output was dropped.
///
/// Override at runtime via `OPS_OUTPUT_BYTE_CAP` (parses as a u64; values
/// `<=0` are ignored and fall back to the default).
pub const DEFAULT_OUTPUT_BYTE_CAP: usize = 4 * 1024 * 1024; // 4 MiB / stream

const OUTPUT_CAP_ENV: &str = "OPS_OUTPUT_BYTE_CAP";

/// PERF-3 / TASK-0542: resolve the env-driven cap once per process. The
/// value is process-global and constant for a run, so the prior per-spawn
/// `std::env::var` lookup contended on the global env lock under
/// `MAX_PARALLEL` parallel commands. `OnceLock` keeps the override /
/// fallback semantics (parsed at first use) without re-reading.
static OUTPUT_BYTE_CAP: OnceLock<usize> = OnceLock::new();

fn output_byte_cap() -> usize {
    *OUTPUT_BYTE_CAP.get_or_init(|| {
        std::env::var(OUTPUT_CAP_ENV)
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|&n| n > 0)
            .unwrap_or(DEFAULT_OUTPUT_BYTE_CAP)
    })
}

/// PERF-1 / TASK-0515: truncate `bytes` to a UTF-8-lossy `String` capped at
/// `cap` bytes. Drops trailing bytes and appends a marker line so the
/// truncation is user-visible. The cap is applied to the byte stream
/// **before** UTF-8 decoding, then the truncation is rounded down to a
/// valid UTF-8 boundary so the marker is never inserted in the middle of a
/// multibyte codepoint.
fn truncate_lossy(bytes: &[u8], cap: usize) -> String {
    if bytes.len() <= cap {
        return String::from_utf8_lossy(bytes).into_owned();
    }
    // Round `cap` down to a UTF-8 char boundary on the lossy decoded output.
    let head_lossy = String::from_utf8_lossy(&bytes[..cap]);
    let mut head = head_lossy.into_owned();
    // After lossy decode any partial codepoint at the tail is already a
    // U+FFFD; nothing more to round. Append a marker so the consumer sees
    // truncation rather than silent drop. PERF-1 / TASK-0577: append via
    // `write!` directly into `head` — the prior `format!(...)` allocated a
    // throwaway String per truncated stream on a per-step hot path.
    let dropped = bytes.len() - cap;
    if !head.ends_with('\n') {
        head.push('\n');
    }
    use std::fmt::Write as _;
    let _ = writeln!(
        &mut head,
        "[ops] output truncated: dropped {dropped} bytes (cap {cap})"
    );
    head
}

impl CommandOutput {
    pub fn from_raw(output: std::process::Output) -> Self {
        let success = output.status.success();
        let cap = output_byte_cap();
        Self {
            success,
            stdout: truncate_lossy(&output.stdout, cap),
            stderr: truncate_lossy(&output.stderr, cap),
            status_message: output.status.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ops_core::test_utils::make_test_output;

    #[test]
    fn command_output_from_raw_success() {
        let output = make_test_output(0, b"hello world", b"");
        let cmd_output = CommandOutput::from_raw(output);
        assert!(cmd_output.success);
        assert_eq!(cmd_output.stdout, "hello world");
        assert!(cmd_output.stderr.is_empty());
    }

    #[test]
    fn command_output_from_raw_failure() {
        let output = make_test_output(1, b"", b"error message");
        let cmd_output = CommandOutput::from_raw(output);
        assert!(!cmd_output.success);
        assert!(cmd_output.stdout.is_empty());
        assert_eq!(cmd_output.stderr, "error message");
    }

    #[test]
    fn command_output_from_raw_invalid_utf8() {
        let output = make_test_output(0, b"hello\xffworld", b"test\xfe");
        let cmd_output = CommandOutput::from_raw(output);
        assert!(cmd_output.stdout.contains("hello"));
        assert!(cmd_output.stdout.contains("world"));
        assert!(cmd_output.stderr.contains("test"));
    }

    #[test]
    fn command_output_status_message() {
        let output = make_test_output(0, b"", b"");
        let cmd_output = CommandOutput::from_raw(output);
        assert!(!cmd_output.status_message.is_empty());
    }

    /// TQ-009: StepResult clone produces equal copy.
    #[test]
    fn step_result_clone_produces_equal_copy() {
        let original = StepResult {
            id: "test".into(),
            success: true,
            duration: Duration::from_secs(5),
            stdout: "output".to_string(),
            stderr: "err".to_string(),
            message: Some("msg".to_string()),
        };
        let cloned = original.clone();
        assert_eq!(cloned.id, original.id);
        assert_eq!(cloned.success, original.success);
        assert_eq!(cloned.duration, original.duration);
        assert_eq!(cloned.stdout, original.stdout);
        assert_eq!(cloned.stderr, original.stderr);
        assert_eq!(cloned.message, original.message);
    }

    /// TQ-009: StepResult Debug output includes key fields.
    #[test]
    fn step_result_debug_includes_fields() {
        let result = StepResult::failure("debug_cmd", Duration::from_millis(42), "oops".into());
        let debug = format!("{:?}", result);
        assert!(debug.contains("debug_cmd"), "Debug should include id");
        assert!(debug.contains("oops"), "Debug should include message");
        assert!(
            debug.contains("false"),
            "Debug should include success=false"
        );
    }

    /// PERF-1 / TASK-0515: a stream larger than the cap must be truncated
    /// and end with the user-visible marker line. The first chunk of the
    /// stream is preserved (head retention).
    ///
    /// PERF-3 / TASK-0542: the cap is now memoized via `OnceLock`, so this
    /// test exercises `truncate_lossy` directly with an explicit cap rather
    /// than mutating `OPS_OUTPUT_BYTE_CAP` (which would be racy under the
    /// memoization and pollute other tests in the same binary).
    #[test]
    fn truncate_lossy_caps_oversized_input() {
        let huge: Vec<u8> = (0..1024).map(|i| b'a' + (i % 26) as u8).collect();
        let truncated = truncate_lossy(&huge, 32);

        assert!(
            truncated.contains("[ops] output truncated"),
            "expected truncation marker, got: {:?}",
            truncated
        );
        assert!(
            truncated.contains("dropped"),
            "marker should mention dropped byte count"
        );
        assert!(
            truncated.starts_with("abcdefghijklmnopqrstuvwxyzabcdef"),
            "head must be preserved, got prefix: {:?}",
            &truncated[..40.min(truncated.len())]
        );
    }

    /// PERF-3 / TASK-0542: `output_byte_cap` is memoized — the first call's
    /// resolved value sticks across many `from_raw` invocations, even if the
    /// process env changes mid-run. We can't reset `OnceLock`, so we verify
    /// that mutating the env after the first read does not change the cap
    /// observed by subsequent calls.
    #[test]
    fn output_byte_cap_is_memoized_across_calls() {
        let first = output_byte_cap();
        // SAFETY: tests under `cargo test` run on a single thread per binary.
        let prev = std::env::var(OUTPUT_CAP_ENV).ok();
        unsafe { std::env::set_var(OUTPUT_CAP_ENV, "1") };
        for _ in 0..100 {
            assert_eq!(output_byte_cap(), first, "cap must not change post-init");
        }
        unsafe {
            match prev {
                Some(v) => std::env::set_var(OUTPUT_CAP_ENV, v),
                None => std::env::remove_var(OUTPUT_CAP_ENV),
            }
        }
        // Sanity-check from_raw goes through the same memoized path.
        let _ = CommandOutput::from_raw(make_test_output(0, b"x", b"y"));
        assert_eq!(output_byte_cap(), first);
    }

    #[test]
    fn command_output_under_cap_is_unchanged() {
        let output = make_test_output(0, b"short stdout", b"short stderr");
        let cmd_output = CommandOutput::from_raw(output);
        assert_eq!(cmd_output.stdout, "short stdout");
        assert_eq!(cmd_output.stderr, "short stderr");
    }

    #[test]
    fn step_result_failure_constructor() {
        let duration = Duration::from_millis(100);
        let result = StepResult::failure("test_cmd", duration, "error occurred".to_string());
        assert_eq!(result.id, "test_cmd");
        assert!(!result.success);
        assert_eq!(result.duration, duration);
        assert!(result.stdout.is_empty());
        assert!(result.stderr.is_empty());
        assert_eq!(result.message, Some("error occurred".to_string()));
    }
}
