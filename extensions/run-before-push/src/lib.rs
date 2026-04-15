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

    #[test]
    fn should_skip_returns_false_by_default() {
        std::env::remove_var(SKIP_ENV_VAR);
        assert!(!should_skip());
    }

    // -- find_git_dir --

    #[test]
    fn find_git_dir_in_current() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(dir.path().join(".git")).unwrap();
        let result = find_git_dir(dir.path());
        assert_eq!(result, Some(dir.path().join(".git")));
    }

    #[test]
    fn find_git_dir_in_parent() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(dir.path().join(".git")).unwrap();
        let sub = dir.path().join("sub");
        std::fs::create_dir(&sub).unwrap();
        let result = find_git_dir(&sub);
        assert_eq!(result, Some(dir.path().join(".git")));
    }

    #[test]
    fn find_git_dir_not_found() {
        let dir = tempfile::tempdir().expect("tempdir");
        let result = find_git_dir(dir.path());
        assert!(result.is_none());
    }

    // -- install_hook --

    #[test]
    fn install_hook_creates_executable_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let git_dir = dir.path().join(".git");
        std::fs::create_dir(&git_dir).unwrap();

        let mut buf = Vec::new();
        let path = install_hook(&git_dir, &mut buf).expect("install_hook");

        assert!(path.exists());
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("ops run-before-push"));

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert!(mode & 0o111 != 0, "hook should be executable");
        }

        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("Installed hook"));
    }

    #[test]
    fn install_hook_idempotent_when_ops_hook_exists() {
        let dir = tempfile::tempdir().expect("tempdir");
        let git_dir = dir.path().join(".git");
        std::fs::create_dir_all(git_dir.join("hooks")).unwrap();
        std::fs::write(git_dir.join("hooks/pre-push"), HOOK_SCRIPT).unwrap();

        let mut buf = Vec::new();
        let path = install_hook(&git_dir, &mut buf).expect("install_hook");

        assert!(path.exists());
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("already installed"));
    }

    #[test]
    fn install_hook_updates_outdated_ops_hook() {
        let dir = tempfile::tempdir().expect("tempdir");
        let git_dir = dir.path().join(".git");
        std::fs::create_dir_all(git_dir.join("hooks")).unwrap();
        std::fs::write(
            git_dir.join("hooks/pre-push"),
            "#!/bin/sh\necho old\nops run-before-push\n",
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

    #[test]
    fn install_hook_refuses_foreign_hook() {
        let dir = tempfile::tempdir().expect("tempdir");
        let git_dir = dir.path().join(".git");
        std::fs::create_dir_all(git_dir.join("hooks")).unwrap();
        std::fs::write(git_dir.join("hooks/pre-push"), "#!/bin/sh\necho foreign\n").unwrap();

        let mut buf = Vec::new();
        let result = install_hook(&git_dir, &mut buf);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not installed by ops"));
    }

    // -- ensure_config_command --

    #[test]
    fn ensure_config_creates_command_in_empty_file() {
        let dir = tempfile::tempdir().expect("tempdir");

        let selected = vec!["verify".to_string()];
        let mut buf = Vec::new();
        ensure_config_command(dir.path(), &selected, &mut buf).expect("ensure_config_command");

        let content = std::fs::read_to_string(dir.path().join(".ops.toml")).unwrap();
        assert!(content.contains("[commands.run-before-push]"));
        assert!(content.contains("verify"));
        assert!(content.contains("fail_fast"));

        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("Added"));
    }

    #[test]
    fn ensure_config_preserves_existing_run_before_push() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join(".ops.toml"),
            "[commands.run-before-push]\ncommands = [\"test\"]\n",
        )
        .unwrap();

        let selected = vec!["verify".to_string()];
        let mut buf = Vec::new();
        ensure_config_command(dir.path(), &selected, &mut buf).expect("ensure_config_command");

        let content = std::fs::read_to_string(dir.path().join(".ops.toml")).unwrap();
        assert!(content.contains(r#"commands = ["test"]"#));

        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("already defined"));
    }

    #[test]
    fn ensure_config_appends_to_existing_config() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join(".ops.toml"),
            "[output]\ntheme = \"compact\"\n\n[commands.build]\nprogram = \"cargo\"\nargs = [\"build\"]\n",
        )
        .unwrap();

        let selected = vec!["build".to_string(), "test".to_string()];
        let mut buf = Vec::new();
        ensure_config_command(dir.path(), &selected, &mut buf).expect("ensure_config_command");

        let content = std::fs::read_to_string(dir.path().join(".ops.toml")).unwrap();
        assert!(content.contains("theme = \"compact\""));
        assert!(content.contains("[commands.build]"));
        assert!(content.contains("[commands.run-before-push]"));
    }

    #[test]
    fn ensure_config_empty_selection_skips() {
        let dir = tempfile::tempdir().expect("tempdir");

        let mut buf = Vec::new();
        ensure_config_command(dir.path(), &[], &mut buf).expect("ensure_config_command");

        assert!(!dir.path().join(".ops.toml").exists());

        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("No commands selected"));
    }

    // -- Extension metadata --

    #[test]
    fn extension_constants() {
        assert_eq!(NAME, "run-before-push");
        assert_eq!(SHORTNAME, "run-before-push");
        assert!(!DESCRIPTION.is_empty());
    }
}
