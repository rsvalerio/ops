//! Subprocess timeout helper shared by install flows.

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
    match child
        .wait_timeout(timeout)
        .context("subprocess wait failed")?
    {
        Some(status) => Ok(status),
        None => {
            let _ = child.kill();
            // Reap the killed child so it does not become a zombie.
            let _ = child.wait();
            anyhow::bail!("{} timed out after {} seconds", label, timeout.as_secs());
        }
    }
}

#[cfg(test)]
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
