//! Subprocess timeout helper shared by install flows.

use anyhow::Context;
use std::time::Duration;

/// Default timeout for cargo/rustup install subprocesses.
pub const DEFAULT_INSTALL_TIMEOUT: Duration = Duration::from_secs(600);

/// Wait for `child` with a polling loop; kill it and bail if it exceeds `timeout`.
pub fn run_with_timeout(
    mut child: std::process::Child,
    timeout: Duration,
    label: &str,
) -> anyhow::Result<std::process::ExitStatus> {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        match child.try_wait().context("subprocess wait failed")? {
            Some(status) => return Ok(status),
            None => {
                if std::time::Instant::now() >= deadline {
                    let _ = child.kill();
                    anyhow::bail!("{} timed out after {} seconds", label, timeout.as_secs());
                }
                std::thread::sleep(Duration::from_millis(200));
            }
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
