//! Bounded-wait subprocess execution.
//!
//! [`run_with_timeout`] runs a `std::process::Command` and returns its
//! captured [`Output`], killing the child and returning a distinct
//! [`RunError::Timeout`] when the wall-clock deadline expires. Shared by the
//! cargo-invoking data providers in `extensions-rust/*` so network-touching
//! cargo subprocesses never hang indefinitely.
//!
//! Per-operation defaults can be overridden with the
//! `OPS_SUBPROCESS_TIMEOUT_SECS` environment variable (see
//! [`default_timeout`]).
//!
//! # Sync-only — async callers must offload
//!
//! [`run_with_timeout`] is a fully synchronous helper: it uses
//! `thread::sleep` + `Child::try_wait` polling at a fixed 100 ms cadence.
//! That cadence is the deliberate resolution ceiling — timeouts may overshoot
//! the requested deadline by up to one poll interval (~100 ms), which is well
//! below the multi-second deadlines all current callers use.
//!
//! Calling this from inside a tokio task would block the runtime worker for
//! up to 100 ms per poll and starve sibling tasks. Today every caller is sync
//! (data providers run from the sync `about` rendering path, never from
//! inside `run_with_runtime`), so no offload is required. Any future async
//! caller MUST wrap the invocation in [`tokio::task::spawn_blocking`] (or
//! introduce a dedicated `tokio::process`-based variant) rather than awaiting
//! it on the runtime thread.

use std::io::{self, Read};
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// Environment variable used to override the per-operation default timeout.
pub const TIMEOUT_ENV: &str = "OPS_SUBPROCESS_TIMEOUT_SECS";

/// Fallback timeout applied when a caller has no operation-specific default
/// and `OPS_SUBPROCESS_TIMEOUT_SECS` is unset or unparseable.
pub const FALLBACK_TIMEOUT: Duration = Duration::from_secs(180);

/// Polling cadence used by [`run_with_timeout`] when waiting on the child.
/// This is the documented resolution ceiling on timeout overshoot.
const POLL_INTERVAL: Duration = Duration::from_millis(100);

/// ASYNC-6 / TASK-0304: upper bound on `OPS_SUBPROCESS_TIMEOUT_SECS`.
///
/// The whole point of [`run_with_timeout`] is bounded execution; allowing an
/// env-driven `u64::MAX` effectively disables the timeout and silently
/// breaks the helper's contract. 1 hour is generous (the longest legitimate
/// caller is `cargo update`, capped well below this) while still preventing
/// an unbounded hang.
pub const MAX_TIMEOUT_SECS: u64 = 3600;

/// Returned when [`run_with_timeout`] has to kill the child because it
/// outran the deadline. The label is the human-readable operation name
/// passed in by the caller (e.g. `"cargo metadata"`).
#[derive(Debug)]
pub struct TimeoutError {
    pub label: String,
    pub timeout: Duration,
}

impl std::fmt::Display for TimeoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} timed out after {}s",
            self.label,
            self.timeout.as_secs()
        )
    }
}

impl std::error::Error for TimeoutError {}

/// Error returned by [`run_with_timeout`]: either the underlying spawn/IO
/// failed, or the child outran the deadline.
#[derive(Debug)]
pub enum RunError {
    Io(io::Error),
    Timeout(TimeoutError),
}

impl std::fmt::Display for RunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunError::Io(e) => write!(f, "{e}"),
            RunError::Timeout(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for RunError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            RunError::Io(e) => Some(e),
            RunError::Timeout(e) => Some(e),
        }
    }
}

impl From<io::Error> for RunError {
    fn from(e: io::Error) -> Self {
        RunError::Io(e)
    }
}

/// Resolve an effective timeout: `OPS_SUBPROCESS_TIMEOUT_SECS` overrides the
/// caller-provided default if present and parses to a non-zero u64; otherwise
/// the operation-specific default is returned unchanged.
///
/// ASYNC-6 / TASK-0304: the override is clamped to [`MAX_TIMEOUT_SECS`] and
/// emits a warning when it had to be clamped, so an accidental
/// `OPS_SUBPROCESS_TIMEOUT_SECS=18446744073709551615` does not silently
/// disable the helper's bounded-wait contract.
#[must_use]
pub fn default_timeout(op_default: Duration) -> Duration {
    let Some(raw) = std::env::var(TIMEOUT_ENV)
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|&s| s > 0)
    else {
        return op_default;
    };
    if raw > MAX_TIMEOUT_SECS {
        tracing::warn!(
            requested = raw,
            clamped_to = MAX_TIMEOUT_SECS,
            env = TIMEOUT_ENV,
            "ASYNC-6: clamping subprocess timeout to upper bound; bounded execution is the helper's contract"
        );
        return Duration::from_secs(MAX_TIMEOUT_SECS);
    }
    Duration::from_secs(raw)
}

/// Run `cmd` with stdout/stderr captured, returning its [`Output`]. Kills
/// the child and returns [`RunError::Timeout`] when the deadline expires.
///
/// `label` is embedded in the timeout error message so callers don't need to
/// wrap the error themselves.
///
/// # Blocking
///
/// Synchronous: polls `Child::try_wait` every 100 ms until the deadline.
/// Timeouts may overshoot by up to one poll interval; that is the documented
/// resolution ceiling. Async callers MUST run this inside
/// `tokio::task::spawn_blocking` — see the module docs.
///
/// # Errors
///
/// Returns [`RunError::Io`] if spawning or waiting on the child fails, and
/// [`RunError::Timeout`] if the child outruns `timeout`.
pub fn run_with_timeout(
    cmd: &mut Command,
    timeout: Duration,
    label: &str,
) -> Result<Output, RunError> {
    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null())
        .spawn()?;

    let stdout_handle = child.stdout.take().map(|mut s| {
        thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = s.read_to_end(&mut buf);
            buf
        })
    });
    let stderr_handle = child.stderr.take().map(|mut s| {
        thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = s.read_to_end(&mut buf);
            buf
        })
    });

    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait()? {
            Some(s) => break s,
            None => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    // Drain pipe readers so background threads terminate.
                    let _ = stdout_handle.and_then(|h| h.join().ok());
                    let _ = stderr_handle.and_then(|h| h.join().ok());
                    return Err(RunError::Timeout(TimeoutError {
                        label: label.to_string(),
                        timeout,
                    }));
                }
                thread::sleep(POLL_INTERVAL);
            }
        }
    };

    let stdout = stdout_handle
        .and_then(|h| h.join().ok())
        .unwrap_or_default();
    let stderr = stderr_handle
        .and_then(|h| h.join().ok())
        .unwrap_or_default();

    Ok(Output {
        status,
        stdout,
        stderr,
    })
}

/// Run `cargo <args...>` in `working_dir` under [`run_with_timeout`].
///
/// `op_default` is the operation-specific timeout; the caller can still
/// override it via `OPS_SUBPROCESS_TIMEOUT_SECS` (handled by
/// [`default_timeout`]). `label` flows through to [`TimeoutError`].
///
/// Centralises the build-Command + run + label pattern shared by
/// `cargo update`, `cargo metadata`, `cargo upgrade`, `cargo deny`, and
/// `cargo llvm-cov` callers in the Rust extensions.
///
/// # Errors
///
/// Returns [`RunError::Io`] if the subprocess fails to spawn and
/// [`RunError::Timeout`] if it outruns the (possibly env-overridden)
/// deadline.
pub fn run_cargo(
    args: &[&str],
    working_dir: &Path,
    op_default: Duration,
    label: &str,
) -> Result<Output, RunError> {
    run_with_timeout(
        Command::new("cargo").args(args).current_dir(working_dir),
        default_timeout(op_default),
        label,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ASYNC-6 / TASK-0304. `default_timeout` reads a process-wide env var,
    /// so timeout-clamp tests must serialize against any other test that
    /// also touches `OPS_SUBPROCESS_TIMEOUT_SECS` (or the raw env). The
    /// `serial_test::serial` attribute below provides that ordering.
    mod env_override {
        use super::*;
        use serial_test::serial;
        use std::time::Duration;

        fn op_default() -> Duration {
            Duration::from_secs(60)
        }

        #[test]
        #[serial]
        fn clamps_huge_value_to_max() {
            std::env::set_var(TIMEOUT_ENV, u64::MAX.to_string());
            let got = default_timeout(op_default());
            std::env::remove_var(TIMEOUT_ENV);
            assert_eq!(got, Duration::from_secs(MAX_TIMEOUT_SECS));
        }

        #[test]
        #[serial]
        fn zero_value_falls_back_to_op_default() {
            std::env::set_var(TIMEOUT_ENV, "0");
            let got = default_timeout(op_default());
            std::env::remove_var(TIMEOUT_ENV);
            assert_eq!(got, op_default());
        }

        #[test]
        #[serial]
        fn unset_returns_op_default() {
            std::env::remove_var(TIMEOUT_ENV);
            let got = default_timeout(op_default());
            assert_eq!(got, op_default());
        }

        #[test]
        #[serial]
        fn within_bounds_is_honored() {
            std::env::set_var(TIMEOUT_ENV, "30");
            let got = default_timeout(op_default());
            std::env::remove_var(TIMEOUT_ENV);
            assert_eq!(got, Duration::from_secs(30));
        }
    }

    #[test]
    fn completes_before_timeout() {
        let out = run_with_timeout(
            Command::new("sh").args(["-c", "printf hello"]),
            Duration::from_secs(5),
            "sh echo",
        )
        .expect("fast command should not time out");
        assert!(out.status.success());
        assert_eq!(out.stdout, b"hello");
    }

    #[test]
    fn fires_timeout_on_hung_subprocess() {
        let err = run_with_timeout(
            Command::new("sh").args(["-c", "sleep 30"]),
            Duration::from_millis(300),
            "sh sleep",
        )
        .expect_err("slow command should time out");
        match err {
            RunError::Timeout(t) => {
                assert_eq!(t.label, "sh sleep");
                assert_eq!(t.timeout, Duration::from_millis(300));
            }
            RunError::Io(e) => panic!("expected timeout, got io error: {e}"),
        }
    }
}
