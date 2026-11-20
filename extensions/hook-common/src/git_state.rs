//! Bounded-wait `git diff --cached` probe shared by hook crates.
//!
//! Lives here rather than in a hook crate so any hook needing the same shape
//! (pre-merge-commit, prepare-commit-msg) shares the bounded-wait,
//! stderr-drain, and env-driven timeout logic.

use std::path::Path;
use std::sync::mpsc::Receiver;
use std::time::Duration;

/// Grace period to drain stderr after `git diff
/// --cached` exits.
const STDERR_DRAIN_GRACE: Duration = Duration::from_millis(500);

/// Most bytes of the child's stderr the drain keeps.
///
/// Only the opening lines of git's stderr are useful in a diagnostic, and
/// the buffer is rendered verbatim into the hook's error output, so a chatty
/// wrapper (or an orphaned grandchild holding the pipe) must not be able to
/// grow it without limit. Bytes past the cap are read and discarded.
pub const MAX_STDERR_BYTES: u64 = 64 * 1024;

/// Marker appended to the captured stderr when the child wrote more than
/// [`MAX_STDERR_BYTES`].
pub const STDERR_TRUNCATED_MARKER: &str = "[stderr truncated]";

/// Typed failure for [`has_staged_files_with_timeout`].
///
/// Callers branch on the variant to decide whether a hook fails or skips,
/// so each variant documents exactly when it is returned.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum HasStagedFilesError {
    /// The probe process could not be started at all: `program` is missing,
    /// not executable, or the spawn itself failed.
    #[error("failed to run `{program} diff --cached`")]
    Spawn {
        /// The program the probe tried to run, as the caller passed it.
        program: String,
        /// The error the spawn returned.
        #[source]
        source: std::io::Error,
    },
    /// The probe started but did not exit within `timeout`; it has been
    /// killed and reaped.
    #[error("`{program} diff --cached` timed out after {timeout:?}")]
    Timeout {
        /// The program the probe ran.
        program: String,
        /// The bounded wait that elapsed.
        timeout: Duration,
    },
    /// The probe exited with a status other than 0 ("nothing staged") or 1
    /// ("staged changes"), such as git's 128 outside a repository.
    #[error("`{program} diff --cached` failed (exit {exit_code:?}): {stderr}")]
    NonZeroExit {
        /// The program the probe ran.
        program: String,
        /// The exit code, or `None` when the child was terminated by a
        /// signal and has no exit code.
        exit_code: Option<i32>,
        /// The child's stderr, decoded as lossy UTF-8 and trimmed. At most
        /// [`MAX_STDERR_BYTES`] of it are kept; longer output ends with
        /// [`STDERR_TRUNCATED_MARKER`]. Empty when the child wrote nothing
        /// within the drain grace period.
        stderr: String,
    },
    /// Waiting on the started probe failed, so its exit status is unknown.
    #[error("failed to wait for `{program} diff --cached`")]
    Io {
        /// The program the probe ran.
        program: String,
        /// The error the wait returned.
        #[source]
        source: std::io::Error,
    },
}

/// Read the env var `env_var` as a u64 number of seconds, clamped to
/// `max_secs`. Returns `None` for unset, zero, or unparseable values
/// (callers fall back to their own default).
///
/// An env-driven effective disable (e.g. `u64::MAX`)
/// would revert the bounded-wait contract, so values past `max_secs` clamp
/// down with a `tracing::warn!` breadcrumb.
pub fn git_timeout_from_env(env_var: &str, max_secs: u64) -> Option<Duration> {
    let Ok(raw) = std::env::var(env_var) else {
        return None;
    };
    match raw.parse::<u64>() {
        Ok(0) | Err(_) => {
            // `raw` is the most directly
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

/// Bounded wait on the stderr drain thread that
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
            // The program name is
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
/// The probe deliberately passes **no** `--diff-filter`, so additions,
/// copies, modifications, renames, deletions (`D`), type changes (`T`) and
/// unmerged paths (`U`) all report as staged work. A filter such as `ACMR`
/// would make a delete-only or conflicted index read as "nothing staged":
/// callers gate a pre-commit check suite on this predicate, so that
/// combination would skip the whole gate with exit 0 on exactly the commits
/// most likely to break a build (a removed module, a removed fixture, a
/// half-resolved merge). A gate must fail closed; a caller that wants a
/// narrower question needs a separate, explicitly named predicate.
///
/// # Bounded wait
///
/// Pre-commit hooks run on the developer's critical path, and an unbounded
/// wait on a hung `git diff --cached` (FUSE-backed worktree, network-mounted
/// `.git`, lock contention) would hang the commit indefinitely. The bounded
/// wait surfaces a typed timeout error so the hook fails loudly instead of
/// silently parking the user's shell.
///
/// Stdout is routed to `/dev/null` (via `--quiet`) and
/// stderr is drained in a worker thread, sidestepping pipe-buffer
/// deadlocks for chatty git wrappers.
///
/// # Single-shot-process only
///
/// The stderr drain thread is fire-and-forget. It
/// blocks on `read_to_end` until the kernel signals EOF on the pipe — i.e.
/// until *every* descriptor inheriting the write end (the child and any
/// orphan grandchild it forked) is closed. After this function returns,
/// the thread, its accumulating `Vec<u8>`, and the pipe FD remain pinned
/// for the lifetime of the longest-lived pipe holder.
///
/// The buffer itself is capped at [`MAX_STDERR_BYTES`]; the thread reads and
/// discards anything past it.
///
/// In a one-shot CLI invocation the cost is bounded by process exit, so
/// pre-commit and friends accept it. **Do not call this from a long-lived
/// host (LSP-style daemon, `ops watch` mode, persistent runner): every
/// hung subprocess pins one drain thread plus one pipe FD plus one
/// capped buffer for the host's lifetime.** A future daemon caller
/// must either close the pipe read end on `wait_timeout` return or move
/// to a non-blocking drain that observes the parent's cancellation.
///
/// # Stderr pipe invariant
///
/// `.stderr(Stdio::piped())` guarantees `child.stderr` is
/// `Some`. The impossible arm drops the sender rather than panicking (see
/// the comment on `spawn_stderr_drain`), which also keeps
/// `read_stderr_bounded` from waiting out its full grace period. This
/// function has no panicking path; it reports every failure as a
/// [`HasStagedFilesError`], per the crate's typed-error policy for hooks.
/// Do not "restore" an `unwrap` here.
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
    let mut child = spawn_diff_cached(program, dir)?;
    let stderr_rx = spawn_stderr_drain(&mut child);
    let status = wait_bounded(&mut child, timeout, program)?;
    let stderr_bytes = read_stderr_bounded(&stderr_rx, STDERR_DRAIN_GRACE, program);
    classify_diff_exit(status, &stderr_bytes, program)
}

/// Spawn `git diff --cached --quiet` in `dir` with the probe's stdio
/// wiring: stdin and stdout to `/dev/null`, stderr piped for the drain
/// thread.
///
/// No `--diff-filter`: see "Every staged change kind counts" on
/// [`has_staged_files_with_timeout`].
fn spawn_diff_cached(
    program: &str,
    dir: &Path,
) -> Result<std::process::Child, HasStagedFilesError> {
    use std::process::{Command, Stdio};

    Command::new(program)
        .current_dir(dir)
        .args(["diff", "--cached", "--quiet"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| HasStagedFilesError::Spawn { program: program.to_string(), source: e })
}

/// Take the child's piped stderr and drain it concurrently, returning the
/// receiver the wait side reads after exit.
///
/// A channel rather than a `JoinHandle` so an orphaned grandchild keeping
/// the pipe open does not stall a blocking `join()`.
///
/// `.stderr(Stdio::piped())` at spawn guarantees `Some`. Dropping the
/// sender in the impossible arm keeps `read_stderr_bounded` from waiting
/// out its full grace period instead of panicking.
fn spawn_stderr_drain(child: &mut std::process::Child) -> Receiver<Vec<u8>> {
    let (stderr_tx, stderr_rx) = std::sync::mpsc::channel::<Vec<u8>>();
    match child.stderr.take() {
        Some(stderr_pipe) => {
            std::thread::spawn(move || {
                let _ = stderr_tx.send(drain_capped(stderr_pipe, MAX_STDERR_BYTES));
            });
        }
        None => drop(stderr_tx),
    }
    stderr_rx
}

/// Read `reader` to EOF, keeping at most `cap` bytes and discarding the rest.
///
/// Draining to EOF (rather than stopping at the cap) keeps the child from
/// blocking on a full pipe. When anything was discarded the kept bytes end
/// with [`STDERR_TRUNCATED_MARKER`] on its own line. Read errors end the
/// drain with whatever was kept, as the uncapped drain did.
fn drain_capped(mut reader: impl std::io::Read, cap: u64) -> Vec<u8> {
    use std::io::Read as _;

    let mut buf = Vec::new();
    let _ = (&mut reader).take(cap).read_to_end(&mut buf);
    let discarded = std::io::copy(&mut reader, &mut std::io::sink()).unwrap_or(0);
    if discarded > 0 {
        buf.push(b'\n');
        buf.extend_from_slice(STDERR_TRUNCATED_MARKER.as_bytes());
    }
    buf
}

/// Wait for the child with a hard deadline, killing it on timeout.
///
/// A single `wait_timeout` syscall returns immediately on a fast
/// `git diff --cached` rather than paying a 50ms busy-poll floor.
fn wait_bounded(
    child: &mut std::process::Child,
    timeout: Duration,
    program: &str,
) -> Result<std::process::ExitStatus, HasStagedFilesError> {
    use wait_timeout::ChildExt;

    match child.wait_timeout(timeout) {
        Ok(Some(status)) => Ok(status),
        Ok(None) => {
            let _ = child.kill();
            let _ = child.wait();
            Err(HasStagedFilesError::Timeout { program: program.to_string(), timeout })
        }
        Err(source) => Err(HasStagedFilesError::Io { program: program.to_string(), source }),
    }
}

/// Classify `git diff --quiet`'s exit status: exit 0 = no staged diff,
/// exit 1 = staged diff present (not an error), other codes = real
/// failure (e.g. not a git repo, which exits 128).
fn classify_diff_exit(
    status: std::process::ExitStatus,
    stderr_bytes: &[u8],
    program: &str,
) -> Result<bool, HasStagedFilesError> {
    match status.code() {
        Some(0) => Ok(false),
        Some(1) => Ok(true),
        _ => {
            let stderr = String::from_utf8_lossy(stderr_bytes).trim().to_string();
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
    fn drain_capped_keeps_short_output_whole() {
        assert_eq!(drain_capped(&b"fatal: boom"[..], 64), b"fatal: boom");
    }

    #[test]
    fn drain_capped_truncates_and_marks_output_past_the_cap() {
        let input = [b'x'; 100];
        let kept = drain_capped(&input[..], 10);
        let expected = format!("{}\n{STDERR_TRUNCATED_MARKER}", "x".repeat(10));
        assert_eq!(kept, expected.as_bytes());
    }

    #[test]
    fn drain_capped_output_exactly_at_the_cap_is_not_marked() {
        let input = vec![b'x'; 10];
        assert_eq!(drain_capped(&input[..], 10), input);
    }

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
        assert_eq!(git_timeout_from_env(TEST_ENV, 300), Some(Duration::from_secs(10)));
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
        assert_eq!(git_timeout_from_env(TEST_ENV, 300), Some(Duration::from_mins(5)));
    }

    /// The unparseable-value warn renders the
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
