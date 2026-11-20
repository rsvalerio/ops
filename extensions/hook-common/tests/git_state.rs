//! Behavioural tests of the shared `git diff --cached` probe,
//! [`has_staged_files_with_timeout`], driven through its public API.
//!
//! Every test here spawns a child process and most hold
//! [`hermetic_git_env`], which mutates the process environment, so every
//! test carries `#[serial_test::serial]`.

// An integration-test target is its own crate (docs/clippy.md layer 2), so
// `allow-unwrap-in-tests` and friends cover only `#[test]` bodies, not the
// fixture helpers below, where a panic on a broken fixture is the desired
// failure mode.
#![allow(clippy::unwrap_used, clippy::panic, reason = "fixture helpers fail the test by panicking")]

use std::assert_matches;
use std::path::{Path, PathBuf};
use std::time::Duration;

use ops_core::test_utils::git_fixture;
use ops_hook_common::git_state::{
    has_staged_files_with_timeout, HasStagedFilesError, MAX_STDERR_BYTES, STDERR_TRUNCATED_MARKER,
};
use ops_hook_common::test_helpers::{hermetic_git_env, EnvGuard};

/// Generous enough for a loaded CI machine; the hang tests pass their own.
const PROBE_TIMEOUT: Duration = Duration::from_secs(5);

fn git(dir: &Path, args: &[&str]) {
    git_fixture(dir, args).unwrap_or_else(|e| panic!("{e}"));
}

fn init_repo(dir: &Path) {
    git(dir, &["init", "-q", "-b", "main"]);
    git(dir, &["config", "user.email", "test@example.com"]);
    git(dir, &["config", "user.name", "Test"]);
}

/// Stage `path` with content and commit it, so a test can then stage a
/// deletion or a type change against a non-empty HEAD.
fn commit_file(dir: &Path, path: &str, contents: &str) {
    std::fs::write(dir.join(path), contents).unwrap();
    git(dir, &["add", path]);
    git(dir, &["commit", "-q", "-m", "seed"]);
}

fn probe(program: &str, dir: &Path) -> Result<bool, HasStagedFilesError> {
    has_staged_files_with_timeout(program, dir, PROBE_TIMEOUT)
}

/// Write an executable fake `git` script into `dir` and return its path.
#[cfg(unix)]
fn write_fake_git(dir: &Path, name: &str, script: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join(name);
    std::fs::write(&path, script).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

/// Run `probe`, retrying while the exec loses the `ETXTBSY` race.
///
/// Writing an executable and exec'ing it straight away is racy under a
/// multi-threaded harness: any thread that forks between our `write` and
/// our `execve` hands the child an inherited write fd, and the kernel
/// answers `execve` on a file that is open for writing with `ETXTBSY`.
/// The window is short, so retry briefly instead of failing the run.
///
/// The retry is sleep-based, so its worst case is 50 x 20 ms = **1 s of
/// added runtime per test**, paid only when the race actually fires.
#[cfg(unix)]
fn retry_while_text_file_busy(
    mut probe: impl FnMut() -> Result<bool, HasStagedFilesError>,
) -> Result<bool, HasStagedFilesError> {
    for _ in 0..50 {
        match probe() {
            Err(HasStagedFilesError::Spawn { source, .. })
                if source.kind() == std::io::ErrorKind::ExecutableFileBusy =>
            {
                std::thread::sleep(Duration::from_millis(20));
            }
            other => return other,
        }
    }
    probe()
}

#[test]
#[serial_test::serial]
fn has_staged_files_false_when_index_empty() {
    let _env = hermetic_git_env();
    let dir = tempfile::tempdir().expect("tempdir");
    init_repo(dir.path());
    assert!(!probe("git", dir.path()).unwrap());
}

#[test]
#[serial_test::serial]
fn has_staged_files_true_when_file_staged() {
    let _env = hermetic_git_env();
    let dir = tempfile::tempdir().expect("tempdir");
    init_repo(dir.path());
    std::fs::write(dir.path().join("a.txt"), "hi").unwrap();
    git(dir.path(), &["add", "a.txt"]);
    assert!(probe("git", dir.path()).unwrap());
}

/// A delete-only index is staged work: a probe filtered to
/// adds/modifications/renames would make `git rm` read as "nothing staged"
/// and skip the whole pre-commit gate with exit 0, on exactly the commits
/// most likely to break a build.
#[test]
#[serial_test::serial]
fn has_staged_files_true_when_only_a_deletion_is_staged() {
    let _env = hermetic_git_env();
    let dir = tempfile::tempdir().expect("tempdir");
    init_repo(dir.path());
    commit_file(dir.path(), "doomed.txt", "bye\n");
    git(dir.path(), &["rm", "-q", "doomed.txt"]);

    assert!(probe("git", dir.path()).unwrap(), "a staged deletion must count as staged work");
}

/// A type change (`T`) is staged work too, alongside deletions and unmerged
/// paths.
#[cfg(unix)]
#[test]
#[serial_test::serial]
fn has_staged_files_true_when_only_a_type_change_is_staged() {
    let _env = hermetic_git_env();
    let dir = tempfile::tempdir().expect("tempdir");
    init_repo(dir.path());
    commit_file(dir.path(), "shifty.txt", "regular\n");

    std::fs::remove_file(dir.path().join("shifty.txt")).unwrap();
    std::os::unix::fs::symlink("elsewhere", dir.path().join("shifty.txt")).unwrap();
    git(dir.path(), &["add", "shifty.txt"]);

    assert!(probe("git", dir.path()).unwrap(), "a staged type change must count as staged work");
}

/// Outside a repository git refuses `diff --cached` with a usage error: exit
/// 129 and an unknown-option error naming `cached` on current git, which falls back to
/// `--no-index` mode there; exit 128 and "not a git repository" on older
/// releases. The probe must report that as `NonZeroExit` carrying git's
/// diagnostic, not as a spawn failure or a timeout.
#[test]
#[serial_test::serial]
fn has_staged_files_errors_outside_git_repo() {
    let _env = hermetic_git_env();
    let dir = tempfile::tempdir().expect("tempdir");
    // Stop git's discovery at the tempdir, so a repository enclosing the
    // temp root cannot turn this into an in-repo probe.
    let _ceiling = EnvGuard::set("GIT_CEILING_DIRECTORIES", dir.path());

    let err = probe("git", dir.path()).unwrap_err();
    assert_matches!(
        &err,
        HasStagedFilesError::NonZeroExit { exit_code: Some(128 | 129), stderr, .. }
            if stderr.contains("unknown option `cached'")
                || stderr.contains("not a git repository")
    );
}

#[cfg(unix)]
#[test]
#[serial_test::serial]
fn has_staged_files_lossily_decodes_invalid_utf8_stderr() {
    let dir = tempfile::tempdir().expect("tempdir");
    let fake_git =
        write_fake_git(dir.path(), "git-fake", "#!/bin/sh\nprintf '\\377\\376' >&2\nexit 128\n");
    let err =
        retry_while_text_file_busy(|| probe(fake_git.to_str().unwrap(), dir.path())).unwrap_err();
    let msg = format!("{err:#}");
    assert!(msg.contains('\u{FFFD}'), "expected lossy U+FFFD in: {msg}");
}

#[test]
#[serial_test::serial]
fn has_staged_files_errors_when_git_binary_missing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let err = probe("git-nonexistent-binary-xyzzy", dir.path()).unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("failed to run"), "unexpected error: {msg}");
    assert_matches!(err, HasStagedFilesError::Spawn { .. });
}

/// A fake git that hangs forever must trigger the bounded-wait timeout
/// rather than blocking the commit indefinitely.
#[cfg(unix)]
#[test]
#[serial_test::serial]
fn has_staged_files_times_out_on_hanging_git() {
    let dir = tempfile::tempdir().expect("tempdir");
    let fake_git = write_fake_git(dir.path(), "git-hang", "#!/bin/sh\nsleep 30\n");

    let started = std::time::Instant::now();
    let err = retry_while_text_file_busy(|| {
        has_staged_files_with_timeout(
            fake_git.to_str().unwrap(),
            dir.path(),
            Duration::from_millis(200),
        )
    })
    .unwrap_err();
    let elapsed = started.elapsed();

    assert_matches!(err, HasStagedFilesError::Timeout { .. });
    // A hang detector, not a performance budget. The fake git sleeps 30 s,
    // so only a bounded wait that never fired can exceed this; 25x the
    // configured 200 ms timeout leaves a loaded machine ample room.
    assert!(
        elapsed < Duration::from_secs(5),
        "the bounded wait did not fire, elapsed = {elapsed:?}"
    );
}

/// Late stderr is captured within the drain grace.
#[cfg(unix)]
#[test]
#[serial_test::serial]
fn has_staged_files_captures_late_stderr_within_drain_grace() {
    let dir = tempfile::tempdir().expect("tempdir");
    let fake_git = write_fake_git(
        dir.path(),
        "git-late-stderr",
        "#!/bin/sh\n\
         sleep 0.1\n\
         printf 'warning: refname HEAD is ambiguous\\nfatal: bad object HEAD\\n' >&2\n\
         exit 128\n",
    );

    let err =
        retry_while_text_file_busy(|| probe(fake_git.to_str().unwrap(), dir.path())).unwrap_err();

    assert_matches!(
        &err,
        HasStagedFilesError::NonZeroExit { exit_code: Some(128), stderr, .. }
            if stderr.contains("fatal: bad object HEAD")
    );
}

/// Large output over the pipe buffer doesn't deadlock.
///
/// The timeout is deliberately generous and there is no wall-clock
/// assertion: a deadlock hangs forever, so any bound distinguishes it from
/// slowness equally well, and the property under test is `Ok(true)`, not
/// machine speed.
#[cfg(unix)]
#[test]
#[serial_test::serial]
fn has_staged_files_handles_large_output_without_deadlock() {
    let dir = tempfile::tempdir().expect("tempdir");
    let fake_git = write_fake_git(
        dir.path(),
        "git-loud",
        "#!/bin/sh\n\
         yes path/to/some/file.txt | head -n 20000\n\
         yes path/to/some/file.txt | head -n 20000 >&2\n\
         exit 1\n",
    );

    let result = retry_while_text_file_busy(|| {
        has_staged_files_with_timeout(
            fake_git.to_str().unwrap(),
            dir.path(),
            Duration::from_secs(30),
        )
    });

    assert_matches!(result, Ok(true));
}

/// A child that floods stderr past [`MAX_STDERR_BYTES`] before failing gets
/// a bounded, marked capture in the error rather than its whole output.
#[cfg(unix)]
#[test]
#[serial_test::serial]
fn has_staged_files_caps_captured_stderr() {
    let dir = tempfile::tempdir().expect("tempdir");
    // 20000 lines of 24 bytes is about 480 KB, well past the 64 KiB cap.
    let fake_git = write_fake_git(
        dir.path(),
        "git-flood",
        "#!/bin/sh\n\
         yes path/to/some/file.txt | head -n 20000 >&2\n\
         exit 128\n",
    );

    let err = retry_while_text_file_busy(|| {
        has_staged_files_with_timeout(
            fake_git.to_str().unwrap(),
            dir.path(),
            Duration::from_secs(30),
        )
    })
    .unwrap_err();

    let HasStagedFilesError::NonZeroExit { stderr, .. } = err else {
        panic!("expected NonZeroExit, got {err:?}");
    };
    let bound = usize::try_from(MAX_STDERR_BYTES).unwrap() + 1 + STDERR_TRUNCATED_MARKER.len();
    assert!(stderr.len() <= bound, "captured {} bytes, bound {bound}", stderr.len());
    assert!(stderr.ends_with(STDERR_TRUNCATED_MARKER), "missing truncation marker");
}
