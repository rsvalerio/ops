//! Run-before-commit hook extension: install and manage git pre-commit hooks.

use std::time::Duration;

use ops_extension::ExtensionType;

// ARCH-1 / TASK-1147: the bounded-wait git-state probe lives in
// `ops_hook_common::git_state` so future hook crates can share it. Re-export
// the public surface here so existing call sites compile unchanged.
pub use ops_hook_common::git_state::{
    has_staged_files_with_timeout, read_stderr_bounded, HasStagedFilesError,
};

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
/// path. The bounded-wait probe lives in `ops_hook_common::git_state`; this
/// crate parameterises it with hook-specific constants.
const DEFAULT_GIT_TIMEOUT: Duration = Duration::from_secs(5);
const TIMEOUT_ENV_VAR: &str = "OPS_RUN_BEFORE_COMMIT_GIT_TIMEOUT_SECS";

/// ASYNC-6 / TASK-0783: upper bound on `OPS_RUN_BEFORE_COMMIT_GIT_TIMEOUT_SECS`.
/// 300 s is generous for even the slowest FUSE-backed worktree while still
/// bounding the hook.
const MAX_GIT_TIMEOUT_SECS: u64 = 300;

/// Returns `true` if there are any staged files in the git index.
pub fn has_staged_files() -> anyhow::Result<bool> {
    use anyhow::Context;
    let cwd = std::env::current_dir().context("failed to read current directory")?;
    let timeout = git_timeout_from_env().unwrap_or(DEFAULT_GIT_TIMEOUT);
    has_staged_files_with_timeout("git", &cwd, timeout).map_err(anyhow::Error::from)
}

fn git_timeout_from_env() -> Option<Duration> {
    ops_hook_common::git_state::git_timeout_from_env(TIMEOUT_ENV_VAR, MAX_GIT_TIMEOUT_SECS)
}

#[cfg(test)]
fn has_staged_files_with(
    program: &str,
    dir: &std::path::Path,
) -> Result<bool, HasStagedFilesError> {
    has_staged_files_with_timeout(program, dir, DEFAULT_GIT_TIMEOUT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ops_hook_common::test_helpers::EnvGuard;
    use std::path::Path;

    // -- HOOK_SCRIPT --

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
        let dir = tempfile::tempdir().expect("tempdir");
        let fake_git = dir.path().join("git-fake");
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
    /// indefinitely.
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
        assert!(
            elapsed < Duration::from_secs(5),
            "timeout should fire promptly, elapsed = {elapsed:?}"
        );
    }

    /// ASYNC-6 / TASK-0864: late stderr captured within drain grace.
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

    /// CONC-3 / TASK-0650 AC#2: large output over pipe buffer doesn't deadlock.
    #[cfg(unix)]
    #[test]
    fn has_staged_files_handles_large_output_without_deadlock() {
        let dir = tempfile::tempdir().expect("tempdir");
        let fake_git = dir.path().join("git-loud");
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
            type Writer = BufWriter;
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
