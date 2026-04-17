//! Run-before-push hook extension: install and manage git pre-push hooks.

use std::io::Write;
use std::path::{Path, PathBuf};

use ops_extension::ExtensionType;
use ops_hook_common::HookConfig;

pub const NAME: &str = "run-before-push";
pub const DESCRIPTION: &str = "Setup git pre-push hook to run an ops command of your choice";
pub const SHORTNAME: &str = "run-before-push";

pub struct RunBeforePushExtension;

ops_extension::impl_extension! {
    RunBeforePushExtension,
    name: NAME,
    description: DESCRIPTION,
    shortname: SHORTNAME,
    types: ExtensionType::COMMAND,
    data_provider_name: None,
    register_data_providers: |_self, _registry| {},
    factory: RUN_BEFORE_PUSH_FACTORY = |_, _| {
        Some((NAME, Box::new(RunBeforePushExtension)))
    },
}

/// The shell script installed as `.git/hooks/pre-push`.
const HOOK_SCRIPT: &str = "#!/usr/bin/env bash\nexec ops run-before-push\n";

/// Environment variable that skips the run-before-push check when set to "1".
pub const SKIP_ENV_VAR: &str = "SKIP_OPS_RUN_BEFORE_PUSH";

/// Hook configuration for the run-before-push extension.
pub fn hook_config() -> HookConfig {
    HookConfig {
        name: NAME,
        hook_filename: "pre-push",
        hook_script: HOOK_SCRIPT,
        skip_env_var: SKIP_ENV_VAR,
        legacy_markers: &["ops run-before-push", "ops before-push"],
        command_help: "Run run-before-push checks before pushing",
    }
}

/// Returns `true` if `SKIP_OPS_RUN_BEFORE_PUSH=1` is set.
pub fn should_skip() -> bool {
    ops_hook_common::should_skip(&hook_config())
}

/// Find the `.git` directory by walking up from the given path.
pub fn find_git_dir(from: &Path) -> Option<PathBuf> {
    ops_hook_common::find_git_dir(from)
}

/// Install the git pre-push hook.
///
/// Returns the path to the created hook file.
pub fn install_hook(git_dir: &Path, w: &mut dyn Write) -> anyhow::Result<PathBuf> {
    ops_hook_common::install_hook(&hook_config(), git_dir, w)
}

/// Ensure a `[commands.run-before-push]` entry exists in `.ops.toml`.
///
/// If the config already has a `run-before-push` command, does nothing.
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
    fn hook_script_contains_ops_run_before_push() {
        assert!(HOOK_SCRIPT.contains("ops run-before-push"));
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
    fn install_hook_updates_legacy_before_push_hook() {
        let dir = tempfile::tempdir().expect("tempdir");
        let git_dir = dir.path().join(".git");
        std::fs::create_dir_all(git_dir.join("hooks")).unwrap();
        std::fs::write(
            git_dir.join("hooks/pre-push"),
            "#!/bin/sh\nexec ops before-push\n",
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
        assert_eq!(NAME, "run-before-push");
        assert_eq!(SHORTNAME, "run-before-push");
        assert!(!DESCRIPTION.is_empty());
    }
}
