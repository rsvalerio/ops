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

/// PERF-1 / TASK-0515 / TASK-0764: per-stream byte cap for captured stdout/stderr.
///
/// A pathological build (e.g. a runaway logger or a `cargo test` flooding
/// stderr) used to balloon the per-step `String` to hundreds of MB. Now the
/// command runner **streams** each pipe and stops buffering once `cap` bytes
/// are in memory; the rest is drained to a sink so the byte count is reported
/// in the truncation marker without keeping the bytes resident. Peak memory
/// per stream is bounded near `cap` regardless of how much the child writes.
///
/// Override at runtime via `OPS_OUTPUT_BYTE_CAP` (parses as a u64; values
/// `<=0` are ignored and fall back to the default).
///
/// **PERF-3 / TASK-0905 — peak-RSS budget for parallel plans.** This cap
/// applies *per spawn × per stream*, so the worst-case in-flight capture
/// budget is `OPS_MAX_PARALLEL × 2 × cap`. With the defaults
/// (`OPS_MAX_PARALLEL=32`, `cap=4 MiB`) that's ≤ 256 MiB; raising
/// `OPS_OUTPUT_BYTE_CAP` to `64M` on a 32-way plan reserves up to 4 GiB.
/// Operators tuning the cap on tight CI runners should also dial down
/// `OPS_MAX_PARALLEL` accordingly. There is intentionally no global
/// semaphore on capture bytes — the per-stream guarantee is the contract,
/// and a global cap would silently truncate output on parallel-plan
/// pressure rather than the documented "first cap bytes are kept"
/// behaviour callers depend on.
pub const DEFAULT_OUTPUT_BYTE_CAP: usize = 4 * 1024 * 1024; // 4 MiB / stream

const OUTPUT_CAP_ENV: &str = "OPS_OUTPUT_BYTE_CAP";

/// PERF-3 / TASK-0542: resolve the env-driven cap once per process. The
/// value is process-global and constant for a run, so the prior per-spawn
/// `std::env::var` lookup contended on the global env lock under
/// `MAX_PARALLEL` parallel commands. `OnceLock` keeps the override /
/// fallback semantics (parsed at first use) without re-reading.
static OUTPUT_BYTE_CAP: OnceLock<usize> = OnceLock::new();

/// PERF-3 / TASK-0905: warn if the per-stream cap × the configured
/// parallel ceiling × 2 streams could reserve more than this many bytes
/// for in-flight captures. Picked at 1 GiB so the default configuration
/// stays silent and only operator-driven escalations trip the alarm.
const PEAK_CAPTURE_WARN_BYTES: usize = 1024 * 1024 * 1024;

/// ERR-2 / TASK-0840: pure parser for the OPS_OUTPUT_BYTE_CAP env value.
/// Returns the resolved cap and, when the input was present-but-unusable,
/// a human message describing the fallback so the caller can emit a
/// `tracing::warn!` outside the unit-test path. Factored out so the
/// fallback semantics are unit-testable without poking the
/// process-global OnceLock.
fn parse_output_byte_cap(raw: Option<&str>) -> (usize, Option<String>) {
    match raw {
        None => (DEFAULT_OUTPUT_BYTE_CAP, None),
        Some(s) => match s.parse::<usize>() {
            Ok(n) if n > 0 => (n, None),
            Ok(_) => (
                DEFAULT_OUTPUT_BYTE_CAP,
                Some(format!(
                    "{OUTPUT_CAP_ENV}={s:?} is not a positive integer; using default {DEFAULT_OUTPUT_BYTE_CAP}"
                )),
            ),
            Err(e) => (
                DEFAULT_OUTPUT_BYTE_CAP,
                Some(format!(
                    "{OUTPUT_CAP_ENV}={s:?} failed to parse as usize ({e}); using default {DEFAULT_OUTPUT_BYTE_CAP}"
                )),
            ),
        },
    }
}

pub(crate) fn output_byte_cap() -> usize {
    *OUTPUT_BYTE_CAP.get_or_init(|| {
        let raw = std::env::var(OUTPUT_CAP_ENV).ok();
        let (cap, warn_msg) = parse_output_byte_cap(raw.as_deref());
        if let Some(msg) = warn_msg {
            // ERR-2 / TASK-0840: one-shot warn naturally because we are
            // inside the OnceLock initialiser. Mirrors OPS_LOG_LEVEL's
            // warn-on-fallback contract in cli/src/main.rs.
            tracing::warn!(env_var = OUTPUT_CAP_ENV, "{msg}");
        }
        // Best-effort: if the operator already overrode OPS_MAX_PARALLEL
        // upward, multiply through and warn when the worst-case in-flight
        // capture budget crosses 1 GiB. The check is informational; we do
        // not clamp `cap` because the per-stream guarantee is the
        // documented contract.
        let max_parallel: usize = std::env::var("OPS_MAX_PARALLEL")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|&n| n > 0)
            .unwrap_or(32);
        let peak = cap.saturating_mul(2).saturating_mul(max_parallel);
        if peak > PEAK_CAPTURE_WARN_BYTES {
            tracing::warn!(
                cap_bytes = cap,
                max_parallel,
                worst_case_bytes = peak,
                threshold_bytes = PEAK_CAPTURE_WARN_BYTES,
                "OPS_OUTPUT_BYTE_CAP × 2 streams × OPS_MAX_PARALLEL exceeds 1 GiB; consider lowering one knob"
            );
        }
        cap
    })
}

impl CommandOutput {
    /// PERF-1 / TASK-0764: build a `CommandOutput` from streamed pipe reads.
    ///
    /// `stdout` / `stderr` carry the head of each stream (already capped at
    /// `cap` by the caller) and `*_dropped` carries the count of bytes that
    /// were drained to a sink past the cap. Used by the streaming exec path
    /// so a misbehaving child writing far more than `cap` to a pipe does not
    /// peak the runner's RSS at the full output size.
    pub fn from_streamed(
        status: std::process::ExitStatus,
        stdout: Vec<u8>,
        stdout_dropped: u64,
        stderr: Vec<u8>,
        stderr_dropped: u64,
    ) -> Self {
        let cap = output_byte_cap();
        Self {
            success: status.success(),
            stdout: cap_streamed(stdout, stdout_dropped, cap),
            stderr: cap_streamed(stderr, stderr_dropped, cap),
            status_message: status.to_string(),
        }
    }
}

/// PERF-1 / TASK-0764: turn a streamed `(head, dropped_after)` pair into the
/// final `String`. Mirrors the marker shape used by `truncate_lossy` so
/// downstream tap consumers see a single canonical truncation line.
fn cap_streamed(mut head: Vec<u8>, dropped_after: u64, cap: usize) -> String {
    let from_head_overflow = head.len().saturating_sub(cap);
    if from_head_overflow > 0 {
        head.truncate(cap);
    }
    let total_dropped = (from_head_overflow as u64).saturating_add(dropped_after);
    let mut out = String::from_utf8_lossy(&head).into_owned();
    if total_dropped == 0 {
        return out;
    }
    if !out.ends_with('\n') {
        out.push('\n');
    }
    use std::fmt::Write as _;
    let _ = writeln!(
        &mut out,
        "[ops] output truncated: dropped {total_dropped} bytes (cap {cap})"
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use ops_core::test_utils::make_test_output;

    fn from_output(output: std::process::Output) -> CommandOutput {
        CommandOutput::from_streamed(output.status, output.stdout, 0, output.stderr, 0)
    }

    #[test]
    fn command_output_from_streamed_success() {
        let output = make_test_output(0, b"hello world", b"");
        let cmd_output = from_output(output);
        assert!(cmd_output.success);
        assert_eq!(cmd_output.stdout, "hello world");
        assert!(cmd_output.stderr.is_empty());
    }

    #[test]
    fn command_output_from_streamed_failure() {
        let output = make_test_output(1, b"", b"error message");
        let cmd_output = from_output(output);
        assert!(!cmd_output.success);
        assert!(cmd_output.stdout.is_empty());
        assert_eq!(cmd_output.stderr, "error message");
    }

    #[test]
    fn command_output_from_streamed_invalid_utf8() {
        let output = make_test_output(0, b"hello\xffworld", b"test\xfe");
        let cmd_output = from_output(output);
        assert!(cmd_output.stdout.contains("hello"));
        assert!(cmd_output.stdout.contains("world"));
        assert!(cmd_output.stderr.contains("test"));
    }

    #[test]
    fn command_output_status_message() {
        let output = make_test_output(0, b"", b"");
        let cmd_output = from_output(output);
        assert!(!cmd_output.status_message.is_empty());
    }

    /// PERF-1 / TASK-0764: when the streaming reader sinks bytes past the cap,
    /// the dropped count flows through `from_streamed` into the marker line
    /// without those bytes ever being held in memory.
    #[test]
    fn from_streamed_marker_reflects_sinked_bytes() {
        let output = make_test_output(0, b"head", b"");
        let cmd_output =
            CommandOutput::from_streamed(output.status, b"head".to_vec(), 9_999, Vec::new(), 0);
        assert!(cmd_output.stdout.starts_with("head"));
        assert!(cmd_output.stdout.contains("dropped 9999 bytes"));
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

    /// PERF-1 / TASK-0764: a stream larger than the cap is reduced to a
    /// `cap`-byte head plus a single marker line. Exercises `cap_streamed`
    /// directly with an explicit cap (avoids racing the memoized
    /// `OPS_OUTPUT_BYTE_CAP`).
    #[test]
    fn cap_streamed_caps_oversized_input() {
        let huge: Vec<u8> = (0..1024).map(|i| b'a' + (i % 26) as u8).collect();
        let truncated = cap_streamed(huge, 0, 32);

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
        // Sanity-check from_streamed goes through the same memoized path.
        let _ = from_output(make_test_output(0, b"x", b"y"));
        assert_eq!(output_byte_cap(), first);
    }

    /// ERR-2 / TASK-0840: parser must surface a fallback warning for
    /// non-positive integers, parse errors, and otherwise stay silent.
    #[test]
    fn parse_output_byte_cap_warns_on_invalid_inputs() {
        // Unset → silent default.
        let (cap, msg) = parse_output_byte_cap(None);
        assert_eq!(cap, DEFAULT_OUTPUT_BYTE_CAP);
        assert!(msg.is_none());

        // Positive integer → silent override.
        let (cap, msg) = parse_output_byte_cap(Some("12345"));
        assert_eq!(cap, 12345);
        assert!(msg.is_none());

        // Zero → fallback + warning mentioning the value and default.
        let (cap, msg) = parse_output_byte_cap(Some("0"));
        assert_eq!(cap, DEFAULT_OUTPUT_BYTE_CAP);
        let m = msg.expect("zero must produce a warn message");
        assert!(m.contains("\"0\""), "msg must quote offending value: {m}");
        assert!(
            m.contains(&DEFAULT_OUTPUT_BYTE_CAP.to_string()),
            "msg must name fallback bytes: {m}"
        );

        // Garbage → fallback + warning naming the parse error.
        let (cap, msg) = parse_output_byte_cap(Some("foo"));
        assert_eq!(cap, DEFAULT_OUTPUT_BYTE_CAP);
        let m = msg.expect("garbage must produce a warn message");
        assert!(m.contains("\"foo\""), "msg must quote offending value: {m}");
        assert!(m.contains("parse"), "msg must mention parse failure: {m}");

        // Negative → parse error path (usize::from_str rejects '-').
        let (cap, msg) = parse_output_byte_cap(Some("-1"));
        assert_eq!(cap, DEFAULT_OUTPUT_BYTE_CAP);
        assert!(msg.is_some());
    }

    #[test]
    fn command_output_under_cap_is_unchanged() {
        let output = make_test_output(0, b"short stdout", b"short stderr");
        let cmd_output = from_output(output);
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
