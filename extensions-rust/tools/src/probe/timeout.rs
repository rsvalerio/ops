//! Timeout-aware wrapper around [`ops_core::subprocess::run_with_timeout`]
//! used by every probe in this module.

use ops_core::subprocess::{default_timeout, run_with_timeout, RunError};
use std::process::Command;
use std::time::Duration;

/// ASYNC-6 / TASK-0914: default deadline for tool/listing probes
/// (`rustup show active-toolchain`, `cargo --list`, `rustup component list
/// --installed`). The whole `ops about` / `ops tools list` UX hangs on
/// these probes, so cap them well under the user's "is this CLI broken?"
/// threshold while still giving rustup time to refresh metadata on a slow
/// network. Override globally via `OPS_SUBPROCESS_TIMEOUT_SECS` — handled
/// by [`default_timeout`].
const PROBE_TIMEOUT: Duration = Duration::from_secs(15);

/// API / TASK-1200: outcome of an installation probe. Carries an explicit
/// `Failed` variant so callers can distinguish a probe that *answered*
/// "tool is not installed" from a probe that *did not answer at all*
/// (timeout, spawn IO failure, non-zero exit). The previous shape
/// (`Option<Output>` mapped to `bool` in callers) collapsed both onto
/// `NotInstalled`, which `install_tool` then re-mediated by reinstalling
/// a perfectly working toolchain.
#[derive(Debug, Clone, Copy)]
pub enum ProbeOutcome<T> {
    Ok(T),
    Failed,
}

impl<T> ProbeOutcome<T> {
    #[allow(dead_code)]
    pub(crate) fn map<U>(self, f: impl FnOnce(T) -> U) -> ProbeOutcome<U> {
        match self {
            ProbeOutcome::Ok(t) => ProbeOutcome::Ok(f(t)),
            ProbeOutcome::Failed => ProbeOutcome::Failed,
        }
    }
}

/// Run a probe Command under [`run_with_timeout`], logging timeout / IO
/// errors at `tracing::warn`. Timeout / spawn / unrecognised-error
/// returns surface as [`ProbeOutcome::Failed`] so the caller can route
/// them through [`crate::ToolStatus::ProbeFailed`] (API / TASK-1200)
/// instead of mis-reporting them as "not installed" and triggering a
/// reinstall.
pub(super) fn run_probe_with_timeout(
    cmd: &mut Command,
    label: &'static str,
) -> ProbeOutcome<std::process::Output> {
    run_probe_with_timeout_inner(cmd, default_timeout(PROBE_TIMEOUT), label)
}

fn run_probe_with_timeout_inner(
    cmd: &mut Command,
    timeout: Duration,
    label: &'static str,
) -> ProbeOutcome<std::process::Output> {
    match run_with_timeout(cmd, timeout, label) {
        Ok(out) => ProbeOutcome::Ok(out),
        Err(RunError::Timeout(e)) => {
            tracing::warn!(
                label,
                timeout_secs = e.timeout.as_secs(),
                "ASYNC-6 / TASK-0914 + API / TASK-1200: probe timed out; reporting tool as ProbeFailed"
            );
            ProbeOutcome::Failed
        }
        Err(RunError::Io(e)) => {
            tracing::warn!(
                label,
                error = %e,
                "probe spawn failed; reporting tool as ProbeFailed"
            );
            ProbeOutcome::Failed
        }
        Err(other) => {
            tracing::warn!(
                label,
                error = %other,
                "probe failed with unrecognized error variant; reporting tool as ProbeFailed"
            );
            ProbeOutcome::Failed
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    /// ASYNC-6 / TASK-0914: prove that the probe wrapper actually honours
    /// the deadline rather than blocking on the child. Calls
    /// `run_probe_with_timeout_inner` with an explicit 1-second timeout so
    /// the test does not depend on `OPS_SUBPROCESS_TIMEOUT_SECS`, whose
    /// resolution is cached process-wide via `OnceLock` and therefore
    /// cannot be reliably mutated from a single test.
    #[test]
    fn timeout_returns_none_quickly() {
        let mut cmd = Command::new("sh");
        cmd.args(["-c", "sleep 30"]);
        let start = std::time::Instant::now();
        let result = run_probe_with_timeout_inner(&mut cmd, Duration::from_secs(1), "sleep test");
        assert!(
            matches!(result, ProbeOutcome::Failed),
            "timeout must surface as ProbeOutcome::Failed"
        );
        assert!(
            start.elapsed() < Duration::from_secs(10),
            "must not hang past the deadline; elapsed = {:?}",
            start.elapsed()
        );
    }
}
