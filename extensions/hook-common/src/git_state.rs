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
    let Ok(raw) = std::env::var(env_var) else {
        return None;
    };
    match raw.parse::<u64>() {
        Ok(0) | Err(_) => {
            // ERR-7 (TASK-0937 / TASK-1886): `raw` is the most directly
            // attacker-supplied string in the crate — it is logged precisely
            // *because* it failed to parse, and this warn lands in the
            // developer's terminal on every commit. Debug-format it so
            // embedded newlines and ANSI escapes cannot forge log lines.
            tracing::warn!(
                env = env_var,
                value = ?raw,
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
            // ERR-7 (TASK-0937 / TASK-1886): the program name is
            // caller-supplied and reaches the same operator-facing stream.
            tracing::debug!(
                program = ?program,
                "stderr drain thread disconnected before sending; using empty stderr"
            );
            Vec::new()
        }
    }
}

/// Returns `true` if `git diff --cached --quiet` reports staged changes
/// in `dir`, with a hard upper bound on wall-clock time.
///
/// # Every staged change kind counts
///
/// SEC-31 / TASK-1903: the probe deliberately passes **no** `--diff-filter`,
/// so additions, copies, modifications, renames, deletions (`D`), type
/// changes (`T`) and unmerged paths (`U`) all report as staged work. It used
/// to filter on `ACMR`, which made a delete-only or conflicted index read as
/// "nothing staged": callers gate a pre-commit check suite on this predicate,
/// so that combination skipped the whole gate with exit 0 on exactly the
/// commits most likely to break a build (a removed module, a removed fixture,
/// a half-resolved merge). A gate must fail closed; if a future caller wants
/// a narrower question, it belongs in a separate, explicitly named predicate.
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
///
/// # Stderr pipe invariant
///
/// READ-4 (TASK-1894): `.stderr(Stdio::piped())` guarantees `child.stderr` is
/// `Some`. The impossible arm drops the sender rather than panicking (see the
/// comment on it), which also keeps `read_stderr_bounded` from waiting out its
/// full grace period. This function has no panicking path; it reports every
/// failure as a [`HasStagedFilesError`], per the crate's typed-error policy
/// for hooks. Do not "restore" an `unwrap` here.
///
/// # Errors
///
/// [`HasStagedFilesError`] if `git` cannot be spawned, does not exit within
/// `timeout`, or exits with a status that is neither "staged files" nor
/// "none staged".
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
        // No `--diff-filter`: see "Every staged change kind counts" above.
        .args(["diff", "--cached", "--quiet"])
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
    let (stderr_tx, stderr_rx) = std::sync::mpsc::channel::<Vec<u8>>();
    match child.stderr.take() {
        Some(mut stderr_pipe) => {
            std::thread::spawn(move || {
                let mut buf = Vec::new();
                let _ = stderr_pipe.read_to_end(&mut buf);
                let _ = stderr_tx.send(buf);
            });
        }
        // `.stderr(Stdio::piped())` above guarantees `Some`. Dropping the
        // sender in the impossible arm keeps `read_stderr_bounded` from
        // waiting out its full grace period instead of panicking.
        None => drop(stderr_tx),
    }

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
            Some(Duration::from_mins(5))
        );
    }

    /// ERR-7 (TASK-0937 / TASK-1886): the unparseable-value warn renders the
    /// raw env value through the `?` formatter, so a value like
    /// `$'10s\nWARN forged log line'` cannot inject a second log line or
    /// rewrite the terminal around it with an ANSI escape. Mirrors
    /// `git::tests::git_pointer_path_debug_escapes_control_characters`, but
    /// asserts on the rendered event rather than the value alone so the
    /// field's sigil itself is pinned.
    #[test]
    #[serial_test::serial]
    fn git_timeout_from_env_warn_escapes_control_characters() {
        let _g = EnvGuard::set(TEST_ENV, "10s\nWARN forged log line\u{1b}[31m");

        let logged = ops_core::test_utils::capture_warn(|| {
            assert_eq!(git_timeout_from_env(TEST_ENV, 300), None);
        });

        let value_line = logged
            .lines()
            .find(|l| l.contains("unparseable or zero value"))
            .unwrap_or_else(|| panic!("expected the warn line, got: {logged}"));
        assert!(
            value_line.contains("\\n"),
            "newline must be escaped, not emitted raw: {value_line}"
        );
        assert!(
            !value_line.contains('\u{1b}'),
            "ANSI escape must not reach the terminal: {value_line}"
        );
        assert!(
            !logged.contains("WARN forged log line\n"),
            "the value must not be able to forge a log line: {logged}"
        );
    }
}
