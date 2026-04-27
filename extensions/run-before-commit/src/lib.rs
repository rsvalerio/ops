//! Run-before-commit hook extension: install and manage git pre-commit hooks.

use std::path::Path;

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

/// Returns `true` if there are any staged files in the git index.
pub fn has_staged_files() -> anyhow::Result<bool> {
    use anyhow::Context;
    let cwd = std::env::current_dir().context("failed to read current directory")?;
    has_staged_files_with("git", &cwd)
}

fn has_staged_files_with(program: &str, dir: &Path) -> anyhow::Result<bool> {
    use anyhow::Context;
    let output = std::process::Command::new(program)
        .current_dir(dir)
        .args(["diff", "--cached", "--name-only", "--diff-filter=ACMR"])
        .output()
        .with_context(|| format!("failed to run `{program} diff --cached`"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "`{program} diff --cached` failed (exit {:?}): {}",
            output.status.code(),
            stderr.trim()
        );
    }
    Ok(!output.stdout.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ops_hook_common::test_helpers::EnvGuard;

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

    #[test]
    fn has_staged_files_errors_when_git_binary_missing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let err = has_staged_files_with("git-nonexistent-binary-xyzzy", dir.path()).unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("failed to run"), "unexpected error: {msg}");
    }

    // -- Extension metadata --

    #[test]
    fn extension_constants() {
        assert_eq!(NAME, "run-before-commit");
        assert_eq!(SHORTNAME, "run-before-commit");
        assert!(!DESCRIPTION.is_empty());
    }
}
