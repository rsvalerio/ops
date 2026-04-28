//! Result types for command execution.

use ops_core::config::CommandId;
use std::time::Duration;

/// Result of running a single command.
#[derive(Debug, Clone)]
#[must_use = "StepResult carries success/failure, stderr, and timing; discarding it hides command failures"]
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

fn output_byte_cap() -> usize {
    std::env::var(OUTPUT_CAP_ENV)
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .filter(|&n| n > 0)
        .unwrap_or(DEFAULT_OUTPUT_BYTE_CAP)
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
    // truncation rather than silent drop.
    let dropped = bytes.len() - cap;
    if !head.ends_with('\n') {
        head.push('\n');
    }
    head.push_str(&format!(
        "[ops] output truncated: dropped {dropped} bytes (cap {cap})\n"
    ));
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
    #[test]
    fn command_output_caps_oversized_stdout() {
        // Set a tiny cap via env so the test does not have to allocate 4
        // MiB. `serial_test` is not pulled in here; we restore the env on
        // every exit path and the test is single-threaded for the
        // duration.
        let prev = std::env::var(OUTPUT_CAP_ENV).ok();
        // SAFETY: tests under `cargo test` run on a single thread per binary.
        unsafe { std::env::set_var(OUTPUT_CAP_ENV, "32") };
        let huge: Vec<u8> = (0..1024).map(|i| b'a' + (i % 26) as u8).collect();
        let output = make_test_output(0, &huge, b"");
        let cmd_output = CommandOutput::from_raw(output);
        // Restore env.
        // SAFETY: same as above.
        unsafe {
            match prev {
                Some(v) => std::env::set_var(OUTPUT_CAP_ENV, v),
                None => std::env::remove_var(OUTPUT_CAP_ENV),
            }
        }

        assert!(
            cmd_output.stdout.contains("[ops] output truncated"),
            "expected truncation marker, got: {:?}",
            cmd_output.stdout
        );
        assert!(
            cmd_output.stdout.contains("dropped"),
            "marker should mention dropped byte count"
        );
        // First 32 bytes must survive (head retention).
        assert!(
            cmd_output
                .stdout
                .starts_with("abcdefghijklmnopqrstuvwxyzabcdef"),
            "head must be preserved, got prefix: {:?}",
            &cmd_output.stdout[..40.min(cmd_output.stdout.len())]
        );
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
