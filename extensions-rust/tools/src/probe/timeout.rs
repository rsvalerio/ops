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

/// Run a probe Command under [`run_with_timeout`], logging timeout / IO
/// errors at `tracing::warn` and returning `None` so the caller can map
/// the failure to `ToolStatus::NotInstalled` without duplicating the
/// logging pattern at every call site.
pub(super) fn run_probe_with_timeout(
    cmd: &mut Command,
    label: &'static str,
) -> Option<std::process::Output> {
    match run_with_timeout(cmd, default_timeout(PROBE_TIMEOUT), label) {
        Ok(out) => Some(out),
        Err(RunError::Timeout(e)) => {
            tracing::warn!(
                label,
                timeout_secs = e.timeout.as_secs(),
                "ASYNC-6 / TASK-0914: probe timed out; reporting tool as not installed"
            );
            None
        }
        Err(RunError::Io(e)) => {
            tracing::warn!(
                label,
                error = %e,
                "probe spawn failed; reporting tool as not installed"
            );
            None
        }
        Err(other) => {
            tracing::warn!(
                label,
                error = %other,
                "probe failed with unrecognized error variant; reporting tool as not installed"
            );
            None
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    /// TEST-18 / TASK-1154: run `body` with the env var `key` set to
    /// `value`, restoring the prior value (or unset) on return. Wrap calls
    /// at the top of an env-mutating test that is also `#[serial_test::serial]`
    /// to keep the unsafe set/restore dance from drifting between sites.
    /// Local to this module — broader cross-crate sharing belongs in
    /// `ops_core::subprocess` if/when a fourth caller appears.
    fn with_env_var<R>(key: &str, value: &str, body: impl FnOnce() -> R) -> R {
        let prev = std::env::var_os(key);
        // SAFETY: callers must hold #[serial_test::serial], guaranteeing
        // no other test thread reads/writes env concurrently.
        unsafe { std::env::set_var(key, value) };
        let result = body();
        match prev {
            Some(v) => unsafe { std::env::set_var(key, v) },
            None => unsafe { std::env::remove_var(key) },
        }
        result
    }

    /// ASYNC-6 / TASK-0914: prove that `run_probe_with_timeout` actually
    /// honours the deadline rather than blocking on the child.
    ///
    /// TEST-18 / TASK-1154: serialised because the body mutates
    /// `OPS_SUBPROCESS_TIMEOUT_SECS`, matching the precedent in
    /// `tools/tests.rs` and `deps/tests.rs` where every env-mutating test
    /// is `#[serial_test::serial]`-guarded. Without this, parallel test
    /// threads racing on the same env var produce flaky timeouts here and
    /// in any other test that reads the variable.
    #[test]
    #[serial_test::serial]
    fn timeout_returns_none_quickly() {
        let mut cmd = Command::new("sh");
        cmd.args(["-c", "sleep 30"]);
        let start = std::time::Instant::now();
        let result = with_env_var(ops_core::subprocess::TIMEOUT_ENV, "1", || {
            run_probe_with_timeout(&mut cmd, "sleep test")
        });
        assert!(result.is_none(), "timeout must surface as None");
        assert!(
            start.elapsed() < std::time::Duration::from_secs(10),
            "must not hang past the deadline; elapsed = {:?}",
            start.elapsed()
        );
    }
}
