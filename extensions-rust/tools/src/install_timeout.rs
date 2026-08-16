//! READ-1 / TASK-1585: install-side subprocess timeout helper. Renamed
//! from `crate::timeout` so the install-side and probe-side timeout
//! helpers no longer share a bare `timeout` module name (the probe-side
//! wrapper lives in [`crate::probe::timeout`]). A future contributor
//! reading `run_with_timeout(...)` can disambiguate via the module
//! path alone. Public symbols (`run_with_timeout`,
//! `DEFAULT_INSTALL_TIMEOUT`) keep their names and are still
//! re-exported from the crate root, so no downstream change is needed.

use anyhow::Context;
use std::time::Duration;
use wait_timeout::ChildExt;

/// Default timeout for cargo/rustup install subprocesses.
pub const DEFAULT_INSTALL_TIMEOUT: Duration = Duration::from_secs(600);

/// Wait for `child` to exit; kill it and bail if it exceeds `timeout`.
///
/// Uses [`wait_timeout`] which sleeps on a platform-native primitive (signalfd /
/// kqueue / WaitForSingleObject) so the calling thread is not woken until the
/// child exits or the deadline elapses. This avoids the 5 Hz busy-poll the old
/// implementation imposed for the entire install duration (up to 10 minutes).
pub fn run_with_timeout(
    mut child: std::process::Child,
    timeout: Duration,
    label: &str,
) -> anyhow::Result<std::process::ExitStatus> {
    if let Some(status) = child
        .wait_timeout(timeout)
        .context("subprocess wait failed")?
    {
        Ok(status)
    } else {
        // ERR-9 / TASK-1584: surface kill/wait failures via
        // `tracing::warn!` instead of swallowing them. A failing
        // `kill()` (e.g. process in an uninterruptible state) leaves
        // a runaway cargo/rustup install eating disk I/O while the
        // bailed error says only "timed out", so post-incident logs
        // need this breadcrumb to explain the next install attempt
        // seeing `already installing` / filesystem-lock contention.
        if let Err(e) = child.kill() {
            tracing::warn!(
                label,
                error = %e,
                pid = child.id(),
                "failed to kill child after timeout; install subprocess may be orphaned"
            );
        }
        // Reap the killed child so it does not become a zombie. Wait
        // can fail if the child was never reaped (e.g. kill above
        // failed) — log so the orphaned-child case stays observable.
        if let Err(e) = child.wait() {
            tracing::warn!(
                label,
                error = %e,
                pid = child.id(),
                "failed to wait on child after timeout; child may not have been reaped"
            );
        }
        anyhow::bail!("{} timed out after {} seconds", label, timeout.as_secs());
    }
}

// TEST-19 / TASK-1591: gated `unix` to match `probe/timeout.rs`'s test
// module — both spawn `sh -c …` to drive a hung subprocess, which is
// not available on Windows by default. The bounded-wait behaviour
// under test is cross-platform; only the harness needs sh.
#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn run_with_timeout_fires_on_hung_subprocess() {
        let child = Command::new("sh")
            .args(["-c", "sleep 60"])
            .spawn()
            .expect("sh must be available");
        let result = run_with_timeout(child, Duration::from_millis(400), "cargo install fake");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("timed out"),
            "expected timeout error, got: {err}"
        );
    }

    #[test]
    fn run_with_timeout_succeeds_for_fast_subprocess() {
        let child = Command::new("sh")
            .args(["-c", "exit 0"])
            .spawn()
            .expect("sh must be available");
        let status = run_with_timeout(child, Duration::from_secs(5), "sh exit 0")
            .expect("fast process should not time out");
        assert!(status.success());
    }
}
