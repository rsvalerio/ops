//! Run-before-commit hook extension: install and manage git pre-commit hooks.

use std::io::Write;
use std::path::{Path, PathBuf};

use ops_extension::ExtensionType;
use ops_hook_common::HookConfig;

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

/// Hook configuration for the run-before-commit extension.
pub fn hook_config() -> HookConfig {
    HookConfig {
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
}

/// Returns `true` if `SKIP_OPS_RUN_BEFORE_COMMIT=1` is set.
pub fn should_skip() -> bool {
    ops_hook_common::should_skip(&hook_config())
}

/// Returns `true` if there are any staged files in the git index.
pub fn has_staged_files() -> anyhow::Result<bool> {
    use anyhow::Context;
    let output = std::process::Command::new("git")
        .args(["diff", "--cached", "--name-only", "--diff-filter=ACMR"])
        .output()
        .context("failed to run git diff --cached")?;
    Ok(!output.stdout.is_empty())
}

/// Find the `.git` directory by walking up from the given path.
pub fn find_git_dir(from: &Path) -> Option<PathBuf> {
    ops_hook_common::find_git_dir(from)
}

/// Install the git pre-commit hook.
///
/// Returns the path to the created hook file.
pub fn install_hook(git_dir: &Path, w: &mut dyn Write) -> anyhow::Result<PathBuf> {
    ops_hook_common::install_hook(&hook_config(), git_dir, w)
}

/// Ensure a `[commands.run-before-commit]` entry exists in `.ops.toml`.
///
/// If the config already has a `run-before-commit` command, does nothing.
/// Otherwise, adds a composite command that runs the given `selected_commands`.
/// If `selected_commands` is empty, skips writing the entry.
pub fn ensure_config_command(
    config_dir: &Path,
    selected_commands: &[String],
    w: &mut dyn Write,
) -> anyhow::Result<()> {
    ops_hook_common::ensure_config_command(&hook_config(), config_dir, selected_commands, w)
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn should_skip_returns_false_by_default() {
        std::env::remove_var(SKIP_ENV_VAR);
        assert!(!should_skip());
    }

    // -- install_hook: wrapper-specific legacy markers --

    #[test]
    fn install_hook_updates_legacy_before_commit_hook() {
        let dir = tempfile::tempdir().expect("tempdir");
        let git_dir = dir.path().join(".git");
        std::fs::create_dir_all(git_dir.join("hooks")).unwrap();
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

    // -- Extension metadata --

    #[test]
    fn extension_constants() {
        assert_eq!(NAME, "run-before-commit");
        assert_eq!(SHORTNAME, "run-before-commit");
        assert!(!DESCRIPTION.is_empty());
    }
}
