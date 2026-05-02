//! Run-before-commit hook extension: install and manage git pre-commit hooks.

use std::path::Path;
use std::time::Duration;

use ops_extension::ExtensionType;

pub const NAME: &str = "run-before-commit";
pub const DESCRIPTION: &str = "Setup git pre-commit hook to run an ops command of your choice";
pub const SHORTNAME: &str = "run-before-commit";

pub struct RunBeforeCommitExtension;

ops_extension::impl_extension! {
    RunBeforeCommitExtension,
    name: NAME,
    description: DESCRIPTION,
    shortname: SHORTNAME,
    types: ExtensionType::COMMAND,
    data_provider_name: None,
    register_data_providers: |_self, _registry| {},
    factory: RUN_BEFORE_COMMIT_FACTORY = |_, _| {
        Some((NAME, Box::new(RunBeforeCommitExtension)))
    },
}

/// The shell script installed as `.git/hooks/pre-commit`.
const HOOK_SCRIPT: &str = "#!/usr/bin/env bash\nexec ops run-before-commit\n";

/// Environment variable that skips the run-before-commit check when set to "1".
pub const SKIP_ENV_VAR: &str = "SKIP_OPS_RUN_BEFORE_COMMIT";

ops_hook_common::impl_hook_wrappers! {
    name: NAME,
    hook_filename: "pre-commit",
    hook_script: HOOK_SCRIPT,
    skip_env_var: SKIP_ENV_VAR,
    legacy_markers: &[
        "ops run-before-commit",
        "ops before-commit",
        "ops pre-commit",
    ],
    command_help: "Run run-before-commit checks before committing",
}

/// ASYNC-6 / TASK-0589: pre-commit hooks run on the developer's critical
/// path. A hung `git diff --cached` (FUSE-backed worktree, network-mounted
/// `.git`, lock contention) used to hang the commit indefinitely. We
/// enforce a bounded wait and surface a typed timeout error so the hook
/// fails loudly instead of silently parking the user's shell.
const DEFAULT_GIT_TIMEOUT: Duration = Duration::from_secs(5);
const TIMEOUT_ENV_VAR: &str = "OPS_RUN_BEFORE_COMMIT_GIT_TIMEOUT_SECS";

/// ASYNC-6 / TASK-0783: upper bound on `OPS_RUN_BEFORE_COMMIT_GIT_TIMEOUT_SECS`.
///
/// Matches the policy from TASK-0304: the bounded-wait contract is the
/// whole point of the pre-commit hook; an env-driven effective disable
/// (e.g. `u64::MAX`) reverts the fix from TASK-0589. 300 s is generous
/// for even the slowest FUSE-backed worktree while still bounding the hook.
const MAX_GIT_TIMEOUT_SECS: u64 = 300;

/// ASYNC-6 / TASK-0864: grace period to drain stderr after `git diff
/// --cached` exits.
///
/// The drain thread copies stderr into a channel and finishes immediately
/// after the child exits cleanly; this bounded wait only matters when a
/// misbehaving wrapper kept the pipe open via an orphan grandchild. Tuning
/// is intentionally **not** plumbed through `OPS_RUN_BEFORE_COMMIT_GIT_TIMEOUT_SECS`:
/// that env var caps the *child wait*, which dominates total hook latency.
/// This grace fires only after the child has already exited, so adding a
/// second knob would just confuse the operator surface for sub-second savings.
/// Bumped from the previous 200 ms so a slow CI host emitting a multi-line
/// git warning right before exit (e.g. fsmonitor / lfs notices) still
/// captures its diagnostic stderr in the `NonZeroExit` error message.
const STDERR_DRAIN_GRACE: Duration = Duration::from_millis(500);

/// Typed failure for `has_staged_files_with`. ASYNC-6 / TASK-0589.
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

/// Returns `true` if there are any staged files in the git index.
pub fn has_staged_files() -> anyhow::Result<bool> {
    use anyhow::Context;
    let cwd = std::env::current_dir().context("failed to read current directory")?;
    let timeout = git_timeout_from_env().unwrap_or(DEFAULT_GIT_TIMEOUT);
    has_staged_files_with_timeout("git", &cwd, timeout).map_err(anyhow::Error::from)
}

fn git_timeout_from_env() -> Option<Duration> {
    let raw = match std::env::var(TIMEOUT_ENV_VAR) {
        Ok(v) => v,
        Err(_) => return None,
    };
    match raw.parse::<u64>() {
        Ok(0) | Err(_) => {
            tracing::warn!(
                env = TIMEOUT_ENV_VAR,
                value = %raw,
                "unparseable or zero value; falling back to default timeout"
            );
            None
        }
        Ok(n) => {
            let clamped = n.min(MAX_GIT_TIMEOUT_SECS);
            if clamped < n {
                tracing::warn!(
                    env = TIMEOUT_ENV_VAR,
                    requested_secs = n,
                    ceiling_secs = MAX_GIT_TIMEOUT_SECS,
                    "clamping to upper bound; bounded execution is the hook's contract"
                );
            }
            Some(Duration::from_secs(clamped))
        }
    }
}

#[cfg(test)]
fn has_staged_files_with(program: &str, dir: &Path) -> Result<bool, HasStagedFilesError> {
    has_staged_files_with_timeout(program, dir, DEFAULT_GIT_TIMEOUT)
}

/// ERR-1 / TASK-0789: bounded wait on the stderr drain thread that
/// distinguishes `Timeout` (expected — drain still running past deadline)
/// from `Disconnected` (drain thread crashed before sending). The
/// disconnect path is the one a future operator chasing an empty stderr
/// needs to know about; logging at `debug` keeps the user-facing happy
/// path quiet while leaving a postmortem breadcrumb.
fn read_stderr_bounded(
    stderr_rx: &std::sync::mpsc::Receiver<Vec<u8>>,
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

fn has_staged_files_with_timeout(
    program: &str,
    dir: &Path,
    timeout: Duration,
) -> Result<bool, HasStagedFilesError> {
    use std::io::Read;
    use std::process::{Command, Stdio};
    use wait_timeout::ChildExt;

    // `--quiet` (implies `--exit-code`) signals presence of a diff via the
    // exit status without writing the path list to stdout, so we route
    // stdout to /dev/null. This sidesteps the pipe-buffer deadlock that
    // would otherwise occur when a large monorepo's staged path list
    // overflows the OS pipe buffer while the parent is in `try_wait`.
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

    // Drain stderr concurrently so a chatty git (or fake binary) cannot
    // fill the stderr pipe buffer and deadlock the wait below. Hand the
    // bytes back through a channel rather than a JoinHandle: an orphaned
    // grandchild (e.g. a `sleep` spawned by a wrapper script) can keep
    // the pipe open after we've killed our direct child, so a blocking
    // `join()` would stall on the grandchild's lifetime. `recv_timeout`
    // gives us a bounded wait either way.
    let mut stderr_pipe = child.stderr.take().expect("stderr piped");
    let (stderr_tx, stderr_rx) = std::sync::mpsc::channel::<Vec<u8>>();
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stderr_pipe.read_to_end(&mut buf);
        let _ = stderr_tx.send(buf);
    });

    // CONC-5 / TASK-0725: a single `wait_timeout` syscall blocks until
    // either the child exits or the deadline expires, so a fast `git diff
    // --cached` (typical <5ms) returns immediately rather than paying the
    // 50ms floor of the previous busy-poll loop. The pre-commit hook runs
    // on every commit, so this latency cut matters on the developer hot
    // path.
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

    // Bounded wait: after a normal exit the drain thread should finish
    // immediately, but a misbehaving wrapper that kept stderr open via an
    // orphan must not be allowed to stall the commit hook. We only use
    // stderr_bytes in the error branch below, so an empty fallback is
    // acceptable.
    // ERR-1 / TASK-0789: distinguish a `Timeout` from `Disconnected`. The
    // timeout case is the bounded-wait we want; a disconnect means the
    // drain thread crashed (e.g. allocator failure on huge stderr) so a
    // postmortem breadcrumb is the only signal a future operator gets when
    // the user-visible error message is empty.
    let stderr_bytes = read_stderr_bounded(&stderr_rx, STDERR_DRAIN_GRACE, program);

    // `git diff --quiet`: exit 0 = no staged diff, exit 1 = staged diff
    // present (not an error), other codes = real failure (e.g. not a git
    // repo, which exits 128).
    match status.code() {
        Some(0) => Ok(false),
        Some(1) => Ok(true),
        _ => {
            // Lossy decoding is intentional: git stderr is overwhelmingly
            // UTF-8 and a readable error (with U+FFFD on the rare bad
            // byte) is more useful to the user than an opaque `[u8]`
            // Debug dump.
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
    use ops_hook_common::test_helpers::EnvGuard;

    // -- HOOK_SCRIPT --

    /// ERR-1 / TASK-0789: dropping the sender before any `send` exercises
    /// the `Disconnected` branch of `read_stderr_bounded`. The function
    /// must return an empty `Vec` (matching the previous behaviour) and
    /// log a debug breadcrumb — pin the byte-level contract so a future
    /// refactor cannot silently swap the variants.
    #[test]
    fn read_stderr_bounded_handles_disconnected_sender() {
        let (tx, rx) = std::sync::mpsc::channel::<Vec<u8>>();
        drop(tx);
        let bytes = super::read_stderr_bounded(&rx, Duration::from_millis(50), "git");
        assert!(bytes.is_empty(), "disconnect must yield empty stderr");
    }

    #[test]
    fn read_stderr_bounded_returns_payload_when_sender_sent() {
        let (tx, rx) = std::sync::mpsc::channel::<Vec<u8>>();
        tx.send(b"boom".to_vec()).unwrap();
        let bytes = super::read_stderr_bounded(&rx, Duration::from_millis(50), "git");
        assert_eq!(bytes, b"boom");
    }

    #[test]
    fn hook_script_contains_ops_run_before_commit() {
        assert!(HOOK_SCRIPT.contains("ops run-before-commit"));
    }

    #[test]
    fn hook_script_starts_with_shebang() {
        assert!(HOOK_SCRIPT.starts_with("#!/usr/bin/env bash"));
    }

    // -- should_skip --

    #[test]
    #[serial_test::serial]
    fn should_skip_returns_false_by_default() {
        let _guard = EnvGuard::remove(SKIP_ENV_VAR);
        assert!(!should_skip());
    }

    // -- install_hook: wrapper-specific legacy markers --

    #[test]
    fn install_hook_updates_legacy_before_commit_hook() {
        let dir = tempfile::tempdir().expect("tempdir");
        let git_dir = dir.path().join(".git");
        std::fs::create_dir_all(git_dir.join("hooks")).unwrap();
        std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").unwrap();
        std::fs::write(
            git_dir.join("hooks/pre-commit"),
            "#!/bin/sh\nexec ops before-commit\n",
        )
        .unwrap();

        let mut buf = Vec::new();
        let path = install_hook(&git_dir, &mut buf).expect("install_hook");

        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, HOOK_SCRIPT);

        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("Updating outdated"));
    }

    #[test]
    fn install_hook_updates_legacy_pre_commit_hook() {
        let dir = tempfile::tempdir().expect("tempdir");
        let git_dir = dir.path().join(".git");
        std::fs::create_dir_all(git_dir.join("hooks")).unwrap();
        std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").unwrap();
        std::fs::write(
            git_dir.join("hooks/pre-commit"),
            "#!/bin/sh\nexec ops pre-commit\n",
        )
        .unwrap();

        let mut buf = Vec::new();
        let path = install_hook(&git_dir, &mut buf).expect("install_hook");

        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, HOOK_SCRIPT);

        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("Updating outdated"));
    }

    // -- has_staged_files --

    fn init_repo(dir: &Path) {
        let status = std::process::Command::new("git")
            .current_dir(dir)
            .args(["init", "-q", "-b", "main"])
            .status()
            .expect("git init");
        assert!(status.success());
        let status = std::process::Command::new("git")
            .current_dir(dir)
            .args(["config", "user.email", "test@example.com"])
            .status()
            .expect("git config email");
        assert!(status.success());
        let status = std::process::Command::new("git")
            .current_dir(dir)
            .args(["config", "user.name", "Test"])
            .status()
            .expect("git config name");
        assert!(status.success());
    }

    #[test]
    fn has_staged_files_false_when_index_empty() {
        let dir = tempfile::tempdir().expect("tempdir");
        init_repo(dir.path());
        assert!(!has_staged_files_with("git", dir.path()).unwrap());
    }

    #[test]
    fn has_staged_files_true_when_file_staged() {
        let dir = tempfile::tempdir().expect("tempdir");
        init_repo(dir.path());
        std::fs::write(dir.path().join("a.txt"), "hi").unwrap();
        let status = std::process::Command::new("git")
            .current_dir(dir.path())
            .args(["add", "a.txt"])
            .status()
            .expect("git add");
        assert!(status.success());
        assert!(has_staged_files_with("git", dir.path()).unwrap());
    }

    #[test]
    fn has_staged_files_errors_outside_git_repo() {
        let dir = tempfile::tempdir().expect("tempdir");
        let err = has_staged_files_with("git", dir.path()).unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("not a git repository") || msg.contains("failed"),
            "unexpected error: {msg}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn has_staged_files_lossily_decodes_invalid_utf8_stderr() {
        // Pin the lossy-decode behavior: invalid UTF-8 bytes from a fake git
        // binary become U+FFFD in the error message rather than aborting the
        // bail with a panic or producing an opaque Debug blob.
        let dir = tempfile::tempdir().expect("tempdir");
        let fake_git = dir.path().join("git-fake");
        // Exit 128 mirrors what real git emits for "not a repository" and
        // similar fatal conditions; under `--quiet` semantics exit 1 is
        // reserved for "diff present" so we cannot reuse it here.
        std::fs::write(&fake_git, "#!/bin/sh\nprintf '\\377\\376' >&2\nexit 128\n").unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&fake_git, std::fs::Permissions::from_mode(0o755)).unwrap();
        let err = has_staged_files_with(fake_git.to_str().unwrap(), dir.path()).unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains('\u{FFFD}'), "expected lossy U+FFFD in: {msg}");
    }

    #[test]
    fn has_staged_files_errors_when_git_binary_missing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let err = has_staged_files_with("git-nonexistent-binary-xyzzy", dir.path()).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("failed to run"), "unexpected error: {msg}");
        assert!(matches!(err, HasStagedFilesError::Spawn { .. }));
    }

    /// ASYNC-6 / TASK-0589 AC#3: a fake git that hangs forever must
    /// trigger the bounded-wait timeout rather than blocking the commit
    /// indefinitely. Uses a short timeout so the test stays fast in CI.
    #[cfg(unix)]
    #[test]
    fn has_staged_files_times_out_on_hanging_git() {
        let dir = tempfile::tempdir().expect("tempdir");
        let fake_git = dir.path().join("git-hang");
        std::fs::write(&fake_git, "#!/bin/sh\nsleep 30\n").unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&fake_git, std::fs::Permissions::from_mode(0o755)).unwrap();

        let started = std::time::Instant::now();
        let err = has_staged_files_with_timeout(
            fake_git.to_str().unwrap(),
            dir.path(),
            Duration::from_millis(200),
        )
        .unwrap_err();
        let elapsed = started.elapsed();

        assert!(
            matches!(err, HasStagedFilesError::Timeout { .. }),
            "expected Timeout variant, got {err:?}"
        );
        // Generous upper bound so a slow CI runner still passes; the key
        // assertion is that we did not block on the 30s sleep.
        assert!(
            elapsed < Duration::from_secs(5),
            "timeout should fire promptly, elapsed = {elapsed:?}"
        );
    }

    /// ASYNC-6 / TASK-0864: a fake git that prints a multi-line stderr
    /// warning ~100 ms before exiting non-zero must still surface the
    /// stderr in the `NonZeroExit` error. Pins the `STDERR_DRAIN_GRACE`
    /// budget so a future shrink does not silently clip the diagnostic.
    #[cfg(unix)]
    #[test]
    fn has_staged_files_captures_late_stderr_within_drain_grace() {
        let dir = tempfile::tempdir().expect("tempdir");
        let fake_git = dir.path().join("git-late-stderr");
        std::fs::write(
            &fake_git,
            "#!/bin/sh\n\
             sleep 0.1\n\
             printf 'warning: refname HEAD is ambiguous\\nfatal: bad object HEAD\\n' >&2\n\
             exit 128\n",
        )
        .unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&fake_git, std::fs::Permissions::from_mode(0o755)).unwrap();

        let err = has_staged_files_with_timeout(
            fake_git.to_str().unwrap(),
            dir.path(),
            Duration::from_secs(5),
        )
        .unwrap_err();

        match err {
            HasStagedFilesError::NonZeroExit {
                exit_code, stderr, ..
            } => {
                assert_eq!(exit_code, Some(128));
                assert!(
                    stderr.contains("fatal: bad object HEAD"),
                    "expected late stderr captured within drain grace, got: {stderr:?}"
                );
            }
            other => panic!("expected NonZeroExit, got {other:?}"),
        }
    }

    /// CONC-3 / TASK-0650 AC#2: a fake git that emits well over the OS
    /// pipe buffer (~64 KiB on Linux, 16 KiB on macOS) must complete
    /// within the configured timeout instead of deadlocking. Routing
    /// stdout to `/dev/null` (via `--quiet`) and draining stderr in a
    /// thread is what makes this work — verify it stays that way.
    #[cfg(unix)]
    #[test]
    fn has_staged_files_handles_large_output_without_deadlock() {
        let dir = tempfile::tempdir().expect("tempdir");
        let fake_git = dir.path().join("git-loud");
        // ~200 KiB to stdout and ~200 KiB to stderr, then exit 1 (= diff
        // present under --quiet semantics). The fake ignores `--quiet` to
        // simulate the worst case where git would emit despite the flag.
        std::fs::write(
            &fake_git,
            "#!/bin/sh\n\
             yes path/to/some/file.txt | head -n 20000\n\
             yes path/to/some/file.txt | head -n 20000 >&2\n\
             exit 1\n",
        )
        .unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&fake_git, std::fs::Permissions::from_mode(0o755)).unwrap();

        let started = std::time::Instant::now();
        let result = has_staged_files_with_timeout(
            fake_git.to_str().unwrap(),
            dir.path(),
            Duration::from_millis(1500),
        );
        let elapsed = started.elapsed();

        assert!(
            matches!(result, Ok(true)),
            "expected Ok(true), got {result:?}"
        );
        assert!(
            elapsed < Duration::from_secs(2),
            "should not deadlock on full pipe buffers, elapsed = {elapsed:?}"
        );
    }

    // -- Extension metadata --

    /// TEST-11 / TASK-0720: pin the public identifier against external
    /// sources of truth instead of comparing the const to a literal copy of
    /// itself. The structural checks here verify (a) the hook script
    /// dispatches to `ops <NAME>` (so a rename of NAME without updating the
    /// embedded script breaks the hook), (b) SHORTNAME tracks NAME so the
    /// CLI alias surface is in sync, and (c) NAME matches the documented
    /// kebab-case shape.
    #[test]
    fn extension_constants() {
        assert!(
            HOOK_SCRIPT.contains(&format!("ops {NAME}")),
            "HOOK_SCRIPT must dispatch to `ops {NAME}`, got: {HOOK_SCRIPT}"
        );
        assert_eq!(SHORTNAME, NAME, "shortname must track NAME");
        assert!(
            NAME.chars().all(|c| c.is_ascii_lowercase() || c == '-')
                && NAME.starts_with(|c: char| c.is_ascii_lowercase()),
            "NAME must be kebab-case, got: {NAME}"
        );
        assert!(!DESCRIPTION.is_empty());
    }

    // -- git_timeout_from_env --

    #[test]
    #[serial_test::serial]
    fn git_timeout_from_env_valid_value() {
        let _guard = EnvGuard::set(TIMEOUT_ENV_VAR, "10");
        assert_eq!(git_timeout_from_env(), Some(Duration::from_secs(10)));
    }

    #[test]
    #[serial_test::serial]
    fn git_timeout_from_env_zero_falls_back() {
        let _guard = EnvGuard::set(TIMEOUT_ENV_VAR, "0");
        assert_eq!(git_timeout_from_env(), None);
    }

    #[test]
    #[serial_test::serial]
    fn git_timeout_from_env_unparseable_falls_back() {
        let _guard = EnvGuard::set(TIMEOUT_ENV_VAR, "10s");
        assert_eq!(git_timeout_from_env(), None);
    }

    #[test]
    #[serial_test::serial]
    fn git_timeout_from_env_unset_returns_none() {
        let _guard = EnvGuard::remove(TIMEOUT_ENV_VAR);
        assert_eq!(git_timeout_from_env(), None);
    }

    /// ASYNC-6 / TASK-0783 AC#2: an overlarge value is clamped, not honoured.
    #[test]
    #[serial_test::serial]
    fn git_timeout_from_env_clamps_to_ceiling() {
        let _guard = EnvGuard::set(TIMEOUT_ENV_VAR, "999999999");
        assert_eq!(
            git_timeout_from_env(),
            Some(Duration::from_secs(MAX_GIT_TIMEOUT_SECS))
        );
    }
}
