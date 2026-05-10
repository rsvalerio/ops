//! Bounded-wait `git diff --cached` probe shared by hook crates.
//!
//! ARCH-1 / TASK-1147: extracted from `run-before-commit/lib.rs` so future
//! hooks needing the same shape (pre-merge-commit, prepare-commit-msg) can
//! reuse the bounded-wait, stderr-drain, and env-driven timeout logic
//! without copy-paste.

use std::path::Path;
use std::sync::mpsc::Receiver;
use std::time::Duration;

/// ASYNC-6 / TASK-0864: grace period to drain stderr after `git diff
/// --cached` exits.
const STDERR_DRAIN_GRACE: Duration = Duration::from_millis(500);

/// Typed failure for [`has_staged_files_with_timeout`]. ASYNC-6 / TASK-0589.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum HasStagedFilesError {
    #[error("failed to run `{program} diff --cached`: {source}")]
    Spawn {
        program: String,
        #[source]
        source: std::io::Error,
    },
    #[error("`{program} diff --cached` timed out after {timeout:?}")]
    Timeout { program: String, timeout: Duration },
    #[error("`{program} diff --cached` failed (exit {exit_code:?}): {stderr}")]
    NonZeroExit {
        program: String,
        exit_code: Option<i32>,
        stderr: String,
    },
    #[error("failed to read output from `{program} diff --cached`: {source}")]
    Io {
        program: String,
        #[source]
        source: std::io::Error,
    },
}

/// Read the env var `env_var` as a u64 number of seconds, clamped to
/// `max_secs`. Returns `None` for unset, zero, or unparseable values
/// (callers fall back to their own default).
///
/// ASYNC-6 / TASK-0783: an env-driven effective disable (e.g. `u64::MAX`)
/// would revert the bounded-wait contract, so values past `max_secs` clamp
/// down with a `tracing::warn!` breadcrumb.
pub fn git_timeout_from_env(env_var: &str, max_secs: u64) -> Option<Duration> {
    let raw = match std::env::var(env_var) {
        Ok(v) => v,
        Err(_) => return None,
    };
    match raw.parse::<u64>() {
        Ok(0) | Err(_) => {
            tracing::warn!(
                env = env_var,
                value = %raw,
                "unparseable or zero value; falling back to default timeout"
            );
            None
        }
        Ok(n) => {
            let clamped = n.min(max_secs);
            if clamped < n {
                tracing::warn!(
                    env = env_var,
                    requested_secs = n,
                    ceiling_secs = max_secs,
                    "clamping to upper bound; bounded execution is the hook's contract"
                );
            }
            Some(Duration::from_secs(clamped))
        }
    }
}

/// ERR-1 / TASK-0789: bounded wait on the stderr drain thread that
/// distinguishes `Timeout` (drain still running past deadline) from
/// `Disconnected` (drain thread crashed before sending).
pub fn read_stderr_bounded(
    stderr_rx: &Receiver<Vec<u8>>,
    timeout: Duration,
    program: &str,
) -> Vec<u8> {
    match stderr_rx.recv_timeout(timeout) {
        Ok(bytes) => bytes,
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => Vec::new(),
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
            tracing::debug!(
                program = %program,
                "stderr drain thread disconnected before sending; using empty stderr"
            );
            Vec::new()
        }
    }
}

/// Returns `true` if `git diff --cached --quiet` reports staged changes
/// in `dir`, with a hard upper bound on wall-clock time.
///
/// ASYNC-6 / TASK-0589: pre-commit hooks run on the developer's critical
/// path. A hung `git diff --cached` (FUSE-backed worktree, network-mounted
/// `.git`, lock contention) used to hang the commit indefinitely. The
/// bounded wait surfaces a typed timeout error so the hook fails loudly
/// instead of silently parking the user's shell.
///
/// CONC-3 / TASK-0650: stdout is routed to `/dev/null` (via `--quiet`) and
/// stderr is drained in a worker thread, sidestepping pipe-buffer
/// deadlocks for chatty git wrappers.
///
/// # Single-shot-process only
///
/// ERR-5 / TASK-1150: the stderr drain thread is fire-and-forget. It
/// blocks on `read_to_end` until the kernel signals EOF on the pipe — i.e.
/// until *every* descriptor inheriting the write end (the child and any
/// orphan grandchild it forked) is closed. After this function returns,
/// the thread, its accumulating `Vec<u8>`, and the pipe FD remain pinned
/// for the lifetime of the longest-lived pipe holder.
///
/// In a one-shot CLI invocation the cost is bounded by process exit, so
/// pre-commit and friends accept it. **Do not call this from a long-lived
/// host (LSP-style daemon, `ops watch` mode, persistent runner): every
/// hung subprocess pins one drain thread plus one pipe FD plus one
/// unbounded buffer for the host's lifetime.** A future daemon caller
/// must either close the pipe read end on `wait_timeout` return or move
/// to a non-blocking drain that observes the parent's cancellation.
pub fn has_staged_files_with_timeout(
    program: &str,
    dir: &Path,
    timeout: Duration,
) -> Result<bool, HasStagedFilesError> {
    use std::io::Read;
    use std::process::{Command, Stdio};
    use wait_timeout::ChildExt;

    let mut child = Command::new(program)
        .current_dir(dir)
        .args(["diff", "--cached", "--quiet", "--diff-filter=ACMR"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| HasStagedFilesError::Spawn {
            program: program.to_string(),
            source: e,
        })?;

    // Drain stderr concurrently so a chatty git cannot fill the pipe
    // buffer and deadlock the wait below. Use a channel rather than a
    // JoinHandle so an orphaned grandchild keeping the pipe open does not
    // stall a blocking `join()`.
    let mut stderr_pipe = child.stderr.take().expect("stderr piped");
    let (stderr_tx, stderr_rx) = std::sync::mpsc::channel::<Vec<u8>>();
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stderr_pipe.read_to_end(&mut buf);
        let _ = stderr_tx.send(buf);
    });

    // CONC-5 / TASK-0725: a single `wait_timeout` syscall returns
    // immediately on a fast `git diff --cached` rather than paying a
    // 50ms busy-poll floor.
    let status = match child.wait_timeout(timeout) {
        Ok(Some(s)) => s,
        Ok(None) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(HasStagedFilesError::Timeout {
                program: program.to_string(),
                timeout,
            });
        }
        Err(e) => {
            return Err(HasStagedFilesError::Io {
                program: program.to_string(),
                source: e,
            });
        }
    };

    let stderr_bytes = read_stderr_bounded(&stderr_rx, STDERR_DRAIN_GRACE, program);

    // `git diff --quiet`: exit 0 = no staged diff, exit 1 = staged diff
    // present (not an error), other codes = real failure (e.g. not a git
    // repo, which exits 128).
    match status.code() {
        Some(0) => Ok(false),
        Some(1) => Ok(true),
        _ => {
            let stderr = String::from_utf8_lossy(&stderr_bytes).trim().to_string();
            Err(HasStagedFilesError::NonZeroExit {
                program: program.to_string(),
                exit_code: status.code(),
                stderr,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::EnvGuard;

    #[test]
    fn read_stderr_bounded_handles_disconnected_sender() {
        let (tx, rx) = std::sync::mpsc::channel::<Vec<u8>>();
        drop(tx);
        let bytes = read_stderr_bounded(&rx, Duration::from_millis(50), "git");
        assert!(bytes.is_empty(), "disconnect must yield empty stderr");
    }

    #[test]
    fn read_stderr_bounded_returns_payload_when_sender_sent() {
        let (tx, rx) = std::sync::mpsc::channel::<Vec<u8>>();
        tx.send(b"boom".to_vec()).unwrap();
        let bytes = read_stderr_bounded(&rx, Duration::from_millis(50), "git");
        assert_eq!(bytes, b"boom");
    }

    const TEST_ENV: &str = "OPS_HOOK_COMMON_GIT_STATE_TEST_TIMEOUT";

    #[test]
    #[serial_test::serial]
    fn git_timeout_from_env_valid_value() {
        let _g = EnvGuard::set(TEST_ENV, "10");
        assert_eq!(
            git_timeout_from_env(TEST_ENV, 300),
            Some(Duration::from_secs(10))
        );
    }

    #[test]
    #[serial_test::serial]
    fn git_timeout_from_env_zero_falls_back() {
        let _g = EnvGuard::set(TEST_ENV, "0");
        assert_eq!(git_timeout_from_env(TEST_ENV, 300), None);
    }

    #[test]
    #[serial_test::serial]
    fn git_timeout_from_env_unparseable_falls_back() {
        let _g = EnvGuard::set(TEST_ENV, "10s");
        assert_eq!(git_timeout_from_env(TEST_ENV, 300), None);
    }

    #[test]
    #[serial_test::serial]
    fn git_timeout_from_env_unset_returns_none() {
        let _g = EnvGuard::remove(TEST_ENV);
        assert_eq!(git_timeout_from_env(TEST_ENV, 300), None);
    }

    #[test]
    #[serial_test::serial]
    fn git_timeout_from_env_clamps_to_ceiling() {
        let _g = EnvGuard::set(TEST_ENV, "999999999");
        assert_eq!(
            git_timeout_from_env(TEST_ENV, 300),
            Some(Duration::from_secs(300))
        );
    }
}
