//! Run-before-commit hook extension: install and manage git pre-commit hooks.

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
///
/// Three properties are load-bearing and covered by tests below:
///
/// 1. **`#!/bin/sh`, not bash** — the body uses nothing bash provides, and a
///    bash dependency breaks the hook on busybox/Alpine images, minimal
///    container builds and BSD/Nix setups without bash in scope, where `env`
///    exits 127 and git blocks every commit with a message naming bash rather
///    than ops (CL-3 / TASK-1910). The legacy hooks this crate recognises and
///    replaces are all `#!/bin/sh` already.
/// 2. **`ops` is probed before it is exec'd** — git hooks inherit the
///    environment of whatever invoked git, and GUI clients (IDE VCS panes,
///    GitHub Desktop, `SourceTree`, Fork) launch from the desktop session, so
///    `~/.cargo/bin` and `~/.local/bin` are routinely absent. A bare
///    `ops: command not found` is a 127 in a dialog box that names neither the
///    tool nor the fix, and the user's only escape is deleting the hook by
///    hand. The guard names ops, the hook path, and the bypass env var.
/// 3. **`--changed-only`** — that flag is what arms the [`has_staged_files`]
///    preflight (`crates/cli/src/subcommands.rs`), so an empty index skips the
///    configured command chain instead of paying for a full check suite.
///    Without it the bounded-wait probe this crate parameterises below is
///    unreachable from the installed hook, and the README's "skips when
///    nothing is staged" is a promise the hook does not keep (ARCH-6 /
///    TASK-1905).
const HOOK_SCRIPT: &str = r#"#!/bin/sh
# Installed by `ops run-before-commit install`.
# The bypass is honoured before the probe below: that probe's own diagnostic
# advertises this variable, so it has to work in exactly the situation the
# diagnostic describes -- ops missing from PATH. Matched with shell builtins
# only, for the same reason. Value list mirrors `ops_hook_common::should_skip`.
case "${SKIP_OPS_RUN_BEFORE_COMMIT:-}" in
    1 | [Tt][Rr][Uu][Ee] | [Yy][Ee][Ss] | [Oo][Nn]) exit 0 ;;
esac
if ! command -v ops >/dev/null 2>&1; then
    echo "pre-commit: cannot find the 'ops' binary on PATH (hook: .git/hooks/pre-commit)." >&2
    echo "pre-commit: add ops to PATH (e.g. ~/.cargo/bin) and rerun \`ops run-before-commit install\`, or bypass with SKIP_OPS_RUN_BEFORE_COMMIT=1." >&2
    exit 1
fi
exec ops run-before-commit --changed-only
"#;

/// Environment variable that skips the run-before-commit check.
///
/// Recognized values are `1`, `true`, `yes` and `on`, matched
/// case-insensitively; anything else — including the empty string, `0` and
/// `false` — means "do not skip". [`ops_hook_common::should_skip`] is the
/// source of truth for that list; keep this doc in step with it
/// (READ-5 / TASK-1916).
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
/// path. The bounded-wait probe lives in `ops_hook_common::git_state`; this
/// crate parameterises it with hook-specific constants.
const DEFAULT_GIT_TIMEOUT: Duration = Duration::from_secs(5);
const TIMEOUT_ENV_VAR: &str = "OPS_RUN_BEFORE_COMMIT_GIT_TIMEOUT_SECS";

/// ASYNC-6 / TASK-0783: upper bound on `OPS_RUN_BEFORE_COMMIT_GIT_TIMEOUT_SECS`.
/// 300 s is generous for even the slowest FUSE-backed worktree while still
/// bounding the hook.
const MAX_GIT_TIMEOUT_SECS: u64 = 300;

/// Returns `true` if the git index holds any staged change.
///
/// Every staged change kind counts — additions, modifications, renames,
/// **deletions**, type changes and unmerged paths — so a delete-only or
/// conflicted index never reads as "nothing staged" and never skips the gate
/// (SEC-31 / TASK-1903). See
/// [`ops_hook_common::git_state::has_staged_files_with_timeout`].
///
/// # Errors
///
/// If the current directory cannot be read, or the `git` probe fails or
/// times out.
pub fn has_staged_files() -> anyhow::Result<bool> {
    use anyhow::Context;
    let cwd = std::env::current_dir().context("failed to read current directory")?;
    let timeout = git_timeout_from_env().unwrap_or(DEFAULT_GIT_TIMEOUT);
    ops_hook_common::git_state::has_staged_files_with_timeout("git", &cwd, timeout)
        .map_err(anyhow::Error::from)
}

fn git_timeout_from_env() -> Option<Duration> {
    ops_hook_common::git_state::git_timeout_from_env(TIMEOUT_ENV_VAR, MAX_GIT_TIMEOUT_SECS)
}

#[cfg(test)]
mod tests {
    // READ-10 / TASK-1917: `clippy.toml` sets `allow-unwrap-in-tests`, so the
    // crate-root `#![cfg_attr(test, allow(...))]` block that used to sit at the
    // top of this file was dead weight — and three of its four entries
    // suppressed cast lints in a crate with no `as` cast at all. Test code is
    // exempt from the panic-adjacent lints by policy, at the narrowest scope
    // clippy offers, with no crate-wide allow to outlive the reason for it.

    use super::*;
    // ARCH-9 / TASK-1915: import the shared probe from its own crate rather
    // than through a re-export from this one — these tests exercise
    // `ops_hook_common`'s bounded wait, not this crate's contribution to it.
    use ops_hook_common::git_state::{has_staged_files_with_timeout, HasStagedFilesError};
    use ops_hook_common::test_helpers::{CwdGuard, EnvGuard};
    use std::path::Path;

    /// Run the shared probe against an explicit `program`/`dir` pair.
    ///
    /// TEST-5 / TASK-1908: this covers **only** `ops_hook_common`'s bounded
    /// wait. It bypasses everything `has_staged_files` itself contributes —
    /// the `current_dir()` lookup, the env-driven timeout, the hardcoded
    /// `"git"`, the `anyhow` conversion — so it is not coverage of the
    /// production preflight. Those four lines are pinned by the
    /// "the production preflight" tests further down.
    fn has_staged_files_with(program: &str, dir: &Path) -> Result<bool, HasStagedFilesError> {
        has_staged_files_with_timeout(program, dir, DEFAULT_GIT_TIMEOUT)
    }

    // -- HOOK_SCRIPT --

    #[test]
    fn hook_script_contains_ops_run_before_commit() {
        assert!(HOOK_SCRIPT.contains("ops run-before-commit"));
    }

    /// CL-3 / TASK-1910 AC#1+#4: the script must not depend on bash being
    /// installed, and no future edit may reintroduce the dependency.
    #[test]
    fn hook_script_uses_posix_sh_shebang() {
        assert!(
            HOOK_SCRIPT.starts_with("#!/bin/sh\n"),
            "HOOK_SCRIPT must not depend on bash, got: {HOOK_SCRIPT}"
        );
        assert!(!HOOK_SCRIPT.contains("bash"));
    }

    /// CL-3 / TASK-1910 AC#2+#3: `ops` is resolved through an explicit probe
    /// that reports what is missing, not by exec'ing it and hoping.
    #[test]
    fn hook_script_guards_missing_ops_binary() {
        assert!(HOOK_SCRIPT.contains("command -v ops"));
        assert!(HOOK_SCRIPT.contains(SKIP_ENV_VAR));
        assert!(HOOK_SCRIPT.contains("exit 1"));
    }

    /// ARCH-6 / TASK-1905 AC#2: the installed hook arms the preflight, so the
    /// README's "skips when nothing is staged" describes the shipped hook.
    #[test]
    fn hook_script_passes_changed_only() {
        assert!(
            HOOK_SCRIPT.contains("exec ops run-before-commit --changed-only"),
            "the installed hook must arm the has_staged_files preflight, got: {HOOK_SCRIPT}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn hook_script_is_valid_posix_sh() {
        let dir = tempfile::tempdir().expect("tempdir");
        let script = dir.path().join("pre-commit");
        std::fs::write(&script, HOOK_SCRIPT).unwrap();
        let status = std::process::Command::new("/bin/sh")
            .arg("-n")
            .arg(&script)
            .status()
            .unwrap();
        assert!(status.success(), "HOOK_SCRIPT must parse under `sh -n`");
    }

    /// CL-3 / TASK-1910 AC#3: with `ops` off PATH the hook must name ops and
    /// the reinstall command on stderr rather than surfacing a bare 127.
    #[cfg(unix)]
    #[test]
    fn hook_script_reports_a_missing_ops_binary_by_name() {
        let dir = tempfile::tempdir().expect("tempdir");
        let script = dir.path().join("pre-commit");
        std::fs::write(&script, HOOK_SCRIPT).unwrap();

        // PATH deliberately excludes the ambient one so a developer's own
        // installed `ops` cannot satisfy the probe.
        let out = std::process::Command::new("/bin/sh")
            .arg(&script)
            .env("PATH", "/usr/bin:/bin")
            .output()
            .unwrap();

        let stderr = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(1), "stderr was: {stderr}");
        assert!(stderr.contains("ops"), "must name ops, got: {stderr}");
        assert!(
            stderr.contains("ops run-before-commit install"),
            "must name the reinstall command, got: {stderr}"
        );
        assert!(
            stderr.contains(SKIP_ENV_VAR),
            "must name the bypass, got: {stderr}"
        );
    }

    /// The missing-ops diagnostic names [`SKIP_ENV_VAR`] as the escape hatch,
    /// so the escape hatch must fire before the probe that prints it —
    /// otherwise the only advice a stuck user gets is advice that does not
    /// work. Driven with `ops` off PATH, which is the situation in question.
    #[cfg(unix)]
    #[test]
    fn hook_script_honours_the_bypass_when_ops_is_missing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let script = dir.path().join("pre-commit");
        std::fs::write(&script, HOOK_SCRIPT).unwrap();

        for value in ["1", "true", "TRUE", "Yes", "on"] {
            let out = std::process::Command::new("/bin/sh")
                .arg(&script)
                .env("PATH", "/usr/bin:/bin")
                .env(SKIP_ENV_VAR, value)
                .output()
                .unwrap();
            assert_eq!(
                out.status.code(),
                Some(0),
                "{value:?} must skip cleanly, stderr was: {}",
                String::from_utf8_lossy(&out.stderr)
            );
        }

        // A value `should_skip` rejects must still reach the probe and fail.
        let out = std::process::Command::new("/bin/sh")
            .arg(&script)
            .env("PATH", "/usr/bin:/bin")
            .env(SKIP_ENV_VAR, "maybe")
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(1));
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

    /// Write an executable fake `git` script into `dir` and return its path.
    #[cfg(unix)]
    fn write_fake_git(dir: &Path, name: &str, script: &str) -> std::path::PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let path = dir.join(name);
        std::fs::write(&path, script).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    /// Run `probe`, retrying while the exec loses the `ETXTBSY` race.
    ///
    /// Writing an executable and exec'ing it straight away is racy under the
    /// parallel test harness: any thread that forks between our `write` and
    /// our `execve` hands the child an inherited write fd, and the kernel
    /// answers `execve` on a file that is open for writing with `ETXTBSY`.
    /// The window is short, so retry briefly instead of failing the run.
    ///
    /// TEST-15 / TASK-1913 AC#4: the retry is sleep-based, so its worst case
    /// is 50 x 20 ms = **1 s of added runtime per test** — paid only when the
    /// race actually fires, which is rare. The deterministic alternative
    /// (closing the write fd before any sibling thread can fork) is not
    /// expressible through `std::fs::write`, so the bound is documented
    /// rather than removed.
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
        let dir = tempfile::tempdir().expect("tempdir");
        init_repo(dir.path());
        assert!(!has_staged_files_with("git", dir.path()).unwrap());
    }

    #[test]
    #[serial_test::serial]
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

    /// Stage `path` with content and commit it, so later tests can stage a
    /// deletion or a type change against a non-empty HEAD.
    fn commit_file(dir: &Path, path: &str, contents: &str) {
        std::fs::write(dir.join(path), contents).unwrap();
        for args in [vec!["add", path], vec!["commit", "-q", "-m", "seed"]] {
            let status = std::process::Command::new("git")
                .current_dir(dir)
                .args(&args)
                .status()
                .expect("git");
            assert!(status.success(), "git {args:?} failed");
        }
    }

    /// SEC-31 / TASK-1903 AC#1+#3: a delete-only index is staged work. The
    /// probe used to filter on `--diff-filter=ACMR`, so `git rm` read as
    /// "nothing staged" and skipped the whole pre-commit gate with exit 0 —
    /// on exactly the commits most likely to break a build.
    #[test]
    #[serial_test::serial]
    fn has_staged_files_true_when_only_a_deletion_is_staged() {
        let dir = tempfile::tempdir().expect("tempdir");
        init_repo(dir.path());
        commit_file(dir.path(), "doomed.txt", "bye\n");

        let status = std::process::Command::new("git")
            .current_dir(dir.path())
            .args(["rm", "-q", "doomed.txt"])
            .status()
            .expect("git rm");
        assert!(status.success());

        assert!(
            has_staged_files_with("git", dir.path()).unwrap(),
            "a staged deletion must count as staged work"
        );
    }

    /// SEC-31 / TASK-1903 AC#2: a type change (`T`) is staged work too — the
    /// old `ACMR` filter excluded it alongside `D` and `U`.
    #[cfg(unix)]
    #[test]
    #[serial_test::serial]
    fn has_staged_files_true_when_only_a_type_change_is_staged() {
        let dir = tempfile::tempdir().expect("tempdir");
        init_repo(dir.path());
        commit_file(dir.path(), "shifty.txt", "regular\n");

        std::fs::remove_file(dir.path().join("shifty.txt")).unwrap();
        std::os::unix::fs::symlink("elsewhere", dir.path().join("shifty.txt")).unwrap();
        let status = std::process::Command::new("git")
            .current_dir(dir.path())
            .args(["add", "shifty.txt"])
            .status()
            .expect("git add");
        assert!(status.success());

        assert!(
            has_staged_files_with("git", dir.path()).unwrap(),
            "a staged type change must count as staged work"
        );
    }

    #[test]
    #[serial_test::serial]
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
        let dir = tempfile::tempdir().expect("tempdir");
        let fake_git = write_fake_git(
            dir.path(),
            "git-fake",
            "#!/bin/sh\nprintf '\\377\\376' >&2\nexit 128\n",
        );
        let err = retry_while_text_file_busy(|| {
            has_staged_files_with(fake_git.to_str().unwrap(), dir.path())
        })
        .unwrap_err();
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
    /// indefinitely.
    #[cfg(unix)]
    #[test]
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

        assert!(
            matches!(err, HasStagedFilesError::Timeout { .. }),
            "expected Timeout variant, got {err:?}"
        );
        // TEST-15 / TASK-1913 AC#3: a hang detector, not a performance
        // budget. The fake git sleeps 30 s, so only a bounded wait that never
        // fired can exceed this — 25x the configured 200 ms timeout leaves a
        // loaded machine ample room.
        assert!(
            elapsed < Duration::from_secs(5),
            "the bounded wait did not fire, elapsed = {elapsed:?}"
        );
    }

    /// ASYNC-6 / TASK-0864: late stderr captured within drain grace.
    #[cfg(unix)]
    #[test]
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

        let err = retry_while_text_file_busy(|| {
            has_staged_files_with_timeout(
                fake_git.to_str().unwrap(),
                dir.path(),
                Duration::from_secs(5),
            )
        })
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

    /// CONC-3 / TASK-0650 AC#2: large output over pipe buffer doesn't deadlock.
    ///
    /// TEST-15 / TASK-1913: the timeout is deliberately generous. A deadlock
    /// hangs forever, so any bound distinguishes it from slowness equally
    /// well — but the old 1500 ms bound also raced the fake git's four forks
    /// and 40 000 lines on a loaded CI box, turning a slow-but-correct run
    /// into `Err(Timeout)` reported as a deadlock regression. The property
    /// under test is `Ok(true)`, not machine speed, so there is no wall-clock
    /// assertion here at all: the timeout itself is the hang detector.
    #[cfg(unix)]
    #[test]
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

        assert!(
            matches!(result, Ok(true)),
            "expected Ok(true), got {result:?}"
        );
    }

    // -- has_staged_files: the production preflight --
    //
    // TEST-5 / TASK-1908: every test above goes through the shared probe with
    // an explicit program and directory. These pin the four lines
    // `has_staged_files` itself contributes — the `current_dir()` lookup, the
    // env-driven timeout reaching the probe, the hardcoded `"git"`, and the
    // `anyhow` conversion — which no test touched before.

    /// The `anyhow`-typed twin of [`retry_while_text_file_busy`], for the
    /// production entry point. Same ETXTBSY race, same worst case (1 s).
    #[cfg(unix)]
    fn retry_anyhow_while_text_file_busy(
        mut probe: impl FnMut() -> anyhow::Result<bool>,
    ) -> anyhow::Result<bool> {
        for _ in 0..50 {
            match probe() {
                Err(e) => match e.downcast_ref::<HasStagedFilesError>() {
                    Some(HasStagedFilesError::Spawn { source, .. })
                        if source.kind() == std::io::ErrorKind::ExecutableFileBusy =>
                    {
                        std::thread::sleep(Duration::from_millis(20));
                    }
                    _ => return Err(e),
                },
                ok => return ok,
            }
        }
        probe()
    }

    /// AC#1: the shipped predicate reads the process working directory.
    #[test]
    #[serial_test::serial]
    fn has_staged_files_reads_the_process_working_directory() {
        let _timeout = EnvGuard::remove(TIMEOUT_ENV_VAR);
        let dir = tempfile::tempdir().expect("tempdir");
        init_repo(dir.path());
        let _cwd = CwdGuard::new(dir.path()).expect("CwdGuard");

        assert!(
            !has_staged_files().expect("empty index probe"),
            "an empty index must report no staged work"
        );

        std::fs::write("a.txt", "hi").unwrap();
        let status = std::process::Command::new("git")
            .args(["add", "a.txt"])
            .status()
            .expect("git add");
        assert!(status.success());

        assert!(
            has_staged_files().expect("staged index probe"),
            "a staged file in the cwd repo must report staged work"
        );
    }

    /// AC#2: the env override is not merely parsed — it is the timeout the
    /// probe actually applies. The assertion reads the timeout back out of
    /// the error rather than timing the call, so it pins the value without
    /// depending on machine speed.
    #[cfg(unix)]
    #[test]
    #[serial_test::serial]
    fn has_staged_files_applies_the_env_timeout() {
        let dir = tempfile::tempdir().expect("tempdir");
        // Named `git`, because `has_staged_files` hardcodes that program name.
        write_fake_git(dir.path(), "git", "#!/bin/sh\nsleep 30\n");

        // `dir` first so the fake `git` shadows the real one; the system bin
        // directories follow because the fake script itself needs `sleep`.
        let _path = EnvGuard::set("PATH", &format!("{}:/usr/bin:/bin", dir.path().display()));
        let _timeout = EnvGuard::set(TIMEOUT_ENV_VAR, "1");
        let _cwd = CwdGuard::new(dir.path()).expect("CwdGuard");

        let err = retry_anyhow_while_text_file_busy(has_staged_files)
            .expect_err("a hanging git must time out");
        let msg = format!("{err:#}");

        assert!(
            msg.contains("timed out after 1s"),
            "the env timeout must be the one applied, got: {msg}"
        );
        assert!(
            !msg.contains(&format!("{DEFAULT_GIT_TIMEOUT:?}")),
            "the default timeout must not win over the env override, got: {msg}"
        );
    }

    /// AC#3: the typed probe error survives the `anyhow` conversion, so the
    /// chain the CLI prints still names what actually failed.
    #[test]
    #[serial_test::serial]
    fn has_staged_files_error_reaches_the_caller_as_an_anyhow_chain() {
        let _timeout = EnvGuard::remove(TIMEOUT_ENV_VAR);
        let dir = tempfile::tempdir().expect("tempdir");
        let _cwd = CwdGuard::new(dir.path()).expect("CwdGuard");

        let err = has_staged_files().expect_err("a non-repo cwd must fail");
        assert!(
            err.downcast_ref::<HasStagedFilesError>().is_some(),
            "the typed probe error must survive into the anyhow chain: {err:#}"
        );
        let msg = format!("{err:#}");
        assert!(
            msg.contains("diff --cached"),
            "the chain must name the probe, got: {msg}"
        );
    }

    // -- Extension metadata --

    /// TEST-11 / TASK-0720: pin the public identifier against external sources of truth.
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

    /// TEST-1 / TASK-0897: capture the WARN emission so a future refactor
    /// that drops the diagnostic while preserving the clamp does not pass
    /// silently.
    mod clamp_log_emission {
        use super::*;
        use std::sync::{Arc, Mutex};
        use tracing_subscriber::fmt::MakeWriter;

        #[derive(Clone, Default)]
        struct BufWriter(Arc<Mutex<Vec<u8>>>);
        impl std::io::Write for BufWriter {
            fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
                self.0.lock().unwrap().extend_from_slice(b);
                Ok(b.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        impl<'a> MakeWriter<'a> for BufWriter {
            type Writer = Self;
            fn make_writer(&'a self) -> Self::Writer {
                self.clone()
            }
        }

        fn capture<F: FnOnce()>(f: F) -> String {
            let buf = BufWriter::default();
            let captured = buf.0.clone();
            let subscriber = tracing_subscriber::fmt()
                .with_writer(buf)
                .with_max_level(tracing::Level::WARN)
                .with_ansi(false)
                .finish();
            tracing::subscriber::with_default(subscriber, f);
            let bytes = captured.lock().unwrap().clone();
            String::from_utf8(bytes).unwrap()
        }

        #[test]
        #[serial_test::serial]
        fn clamps_to_ceiling_emits_warn() {
            let _guard = EnvGuard::set(TIMEOUT_ENV_VAR, "999999999");
            let logs = capture(|| {
                let _ = git_timeout_from_env();
            });
            assert!(logs.contains("WARN"), "expected WARN level, got: {logs}");
            assert!(logs.contains(TIMEOUT_ENV_VAR), "missing env field: {logs}");
            assert!(
                logs.contains("requested_secs"),
                "missing requested_secs field: {logs}"
            );
            assert!(
                logs.contains("ceiling_secs"),
                "missing ceiling_secs field: {logs}"
            );
            assert_eq!(
                logs.matches("clamping to upper bound").count(),
                1,
                "expected exactly one clamp warn, got: {logs}"
            );
        }

        #[test]
        #[serial_test::serial]
        fn at_ceiling_emits_no_warn() {
            let value = MAX_GIT_TIMEOUT_SECS.to_string();
            let _guard = EnvGuard::set(TIMEOUT_ENV_VAR, &value);
            let logs = capture(|| {
                let _ = git_timeout_from_env();
            });
            assert!(
                !logs.contains("clamping to upper bound"),
                "no clamp warn expected at the boundary, got: {logs}"
            );
        }
    }
}
