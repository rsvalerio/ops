//! Shared logic for git hook extensions (run-before-commit, run-before-push).
//!
//! Both hook crates share identical control flow differing only in constants
//! (hook filename, env var name, legacy markers, help text). This crate
//! extracts those common functions behind a [`HookConfig`] descriptor.

use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Context;

/// Describes one git-hook extension so the shared helpers know which file to
/// create, which env var to check, etc.
pub struct HookConfig {
    /// Command name, e.g. `"run-before-commit"`.
    pub name: &'static str,
    /// Git hook filename inside `.git/hooks/`, e.g. `"pre-commit"`.
    pub hook_filename: &'static str,
    /// The full hook script to install.
    pub hook_script: &'static str,
    /// Environment variable that, when set to `"1"`, skips execution.
    pub skip_env_var: &'static str,
    /// Substrings in an existing hook that mark it as a legacy ops hook
    /// (will be overwritten).
    pub legacy_markers: &'static [&'static str],
    /// Help text written into the TOML command entry.
    pub command_help: &'static str,
}

/// Returns `true` if the skip env var is set to `"1"`.
pub fn should_skip(config: &HookConfig) -> bool {
    std::env::var(config.skip_env_var).is_ok_and(|v| v == "1")
}

/// Find the `.git` directory by walking up from the given path.
pub fn find_git_dir(from: &Path) -> Option<PathBuf> {
    let mut dir = from.to_path_buf();
    loop {
        let candidate = dir.join(".git");
        if candidate.is_dir() {
            return Some(candidate);
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// Install the git hook described by `config`.
///
/// Returns the path to the created hook file.
pub fn install_hook(
    config: &HookConfig,
    git_dir: &Path,
    w: &mut dyn Write,
) -> anyhow::Result<PathBuf> {
    let hooks_dir = git_dir.join("hooks");
    std::fs::create_dir_all(&hooks_dir).context("failed to create .git/hooks directory")?;

    let hook_path = hooks_dir.join(config.hook_filename);

    if hook_path.exists() {
        let existing =
            std::fs::read_to_string(&hook_path).context("failed to read existing hook")?;
        if existing == config.hook_script {
            writeln!(w, "Hook already installed at {}", hook_path.display())?;
            return Ok(hook_path);
        }
        if config
            .legacy_markers
            .iter()
            .any(|marker| existing.contains(marker))
        {
            // Old/outdated ops hook — overwrite it below
            writeln!(w, "Updating outdated ops hook at {}", hook_path.display())?;
        } else {
            anyhow::bail!(
                "a {} hook already exists at {} and was not installed by ops. \
                 Remove it manually or back it up before running install.",
                config.hook_filename,
                hook_path.display()
            );
        }
    }

    std::fs::write(&hook_path, config.hook_script).context("failed to write hook")?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&hook_path, std::fs::Permissions::from_mode(0o755))
            .context("failed to make hook executable")?;
    }

    writeln!(w, "Installed hook at {}", hook_path.display())?;
    Ok(hook_path)
}

/// Ensure a `[commands.<name>]` entry exists in `.ops.toml`.
///
/// If the config already has the command, does nothing.
/// Otherwise, adds a composite command that runs the given `selected_commands`.
/// If `selected_commands` is empty, skips writing the entry.
pub fn ensure_config_command(
    config: &HookConfig,
    config_dir: &Path,
    selected_commands: &[String],
    w: &mut dyn Write,
) -> anyhow::Result<()> {
    if selected_commands.is_empty() {
        writeln!(w, "No commands selected; skipping .ops.toml update")?;
        return Ok(());
    }

    let config_path = config_dir.join(".ops.toml");

    let content = if config_path.exists() {
        std::fs::read_to_string(&config_path).context("failed to read .ops.toml")?
    } else {
        String::new()
    };

    let mut doc = content
        .parse::<toml_edit::DocumentMut>()
        .unwrap_or_else(|_| toml_edit::DocumentMut::new());

    // Check if command already exists
    if let Some(commands) = doc.get("commands").and_then(|c| c.as_table()) {
        if commands.contains_key(config.name) {
            writeln!(w, "Command '{}' already defined in .ops.toml", config.name)?;
            return Ok(());
        }
    }

    // Ensure [commands] table exists
    if !doc.contains_key("commands") {
        doc["commands"] = toml_edit::Item::Table(toml_edit::Table::new());
    }

    let commands = doc["commands"]
        .as_table_mut()
        .context("commands is not a table")?;

    let mut cmd = toml_edit::Table::new();

    let mut arr = toml_edit::Array::new();
    for name in selected_commands {
        arr.push(name.as_str());
    }
    cmd.insert("commands", toml_edit::value(arr));
    cmd.insert("fail_fast", toml_edit::value(true));
    cmd.insert("help", toml_edit::value(config.command_help));

    commands.insert(config.name, toml_edit::Item::Table(cmd));

    std::fs::write(&config_path, doc.to_string()).context("failed to write .ops.toml")?;
    writeln!(w, "Added '{}' command to .ops.toml", config.name)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commit_config() -> HookConfig {
        HookConfig {
            name: "run-before-commit",
            hook_filename: "pre-commit",
            hook_script: "#!/usr/bin/env bash\nexec ops run-before-commit\n",
            skip_env_var: "SKIP_OPS_RUN_BEFORE_COMMIT",
            legacy_markers: &[
                "ops run-before-commit",
                "ops before-commit",
                "ops pre-commit",
            ],
            command_help: "Run run-before-commit checks before committing",
        }
    }

    fn push_config() -> HookConfig {
        HookConfig {
            name: "run-before-push",
            hook_filename: "pre-push",
            hook_script: "#!/usr/bin/env bash\nexec ops run-before-push\n",
            skip_env_var: "SKIP_OPS_RUN_BEFORE_PUSH",
            legacy_markers: &["ops run-before-push", "ops before-push"],
            command_help: "Run run-before-push checks before pushing",
        }
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

    // -- should_skip --

    #[test]
    fn should_skip_returns_false_by_default() {
        let cfg = commit_config();
        std::env::remove_var(cfg.skip_env_var);
        assert!(!should_skip(&cfg));
    }

    // -- install_hook (test both configs) --

    #[test]
    fn install_hook_creates_executable_file_commit() {
        let cfg = commit_config();
        let dir = tempfile::tempdir().expect("tempdir");
        let git_dir = dir.path().join(".git");
        std::fs::create_dir(&git_dir).unwrap();

        let mut buf = Vec::new();
        let path = install_hook(&cfg, &git_dir, &mut buf).expect("install_hook");

        assert!(path.exists());
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("ops run-before-commit"));

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
    fn install_hook_creates_executable_file_push() {
        let cfg = push_config();
        let dir = tempfile::tempdir().expect("tempdir");
        let git_dir = dir.path().join(".git");
        std::fs::create_dir(&git_dir).unwrap();

        let mut buf = Vec::new();
        let path = install_hook(&cfg, &git_dir, &mut buf).expect("install_hook");

        assert!(path.exists());
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("ops run-before-push"));

        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("Installed hook"));
    }

    #[test]
    fn install_hook_idempotent_when_ops_hook_exists() {
        let cfg = commit_config();
        let dir = tempfile::tempdir().expect("tempdir");
        let git_dir = dir.path().join(".git");
        std::fs::create_dir_all(git_dir.join("hooks")).unwrap();
        std::fs::write(git_dir.join("hooks/pre-commit"), cfg.hook_script).unwrap();

        let mut buf = Vec::new();
        let path = install_hook(&cfg, &git_dir, &mut buf).expect("install_hook");

        assert!(path.exists());
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("already installed"));
    }

    #[test]
    fn install_hook_updates_outdated_ops_hook() {
        let cfg = commit_config();
        let dir = tempfile::tempdir().expect("tempdir");
        let git_dir = dir.path().join(".git");
        std::fs::create_dir_all(git_dir.join("hooks")).unwrap();
        std::fs::write(
            git_dir.join("hooks/pre-commit"),
            "#!/bin/sh\necho old\nops run-before-commit\n",
        )
        .unwrap();

        let mut buf = Vec::new();
        let path = install_hook(&cfg, &git_dir, &mut buf).expect("install_hook");

        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, cfg.hook_script);

        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("Updating outdated"));
    }

    #[test]
    fn install_hook_updates_legacy_hook() {
        let cfg = commit_config();
        let dir = tempfile::tempdir().expect("tempdir");
        let git_dir = dir.path().join(".git");
        std::fs::create_dir_all(git_dir.join("hooks")).unwrap();
        std::fs::write(
            git_dir.join("hooks/pre-commit"),
            "#!/bin/sh\nexec ops before-commit\n",
        )
        .unwrap();

        let mut buf = Vec::new();
        let path = install_hook(&cfg, &git_dir, &mut buf).expect("install_hook");

        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, cfg.hook_script);

        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("Updating outdated"));
    }

    #[test]
    fn install_hook_refuses_foreign_hook() {
        let cfg = commit_config();
        let dir = tempfile::tempdir().expect("tempdir");
        let git_dir = dir.path().join(".git");
        std::fs::create_dir_all(git_dir.join("hooks")).unwrap();
        std::fs::write(
            git_dir.join("hooks/pre-commit"),
            "#!/bin/sh\necho foreign\n",
        )
        .unwrap();

        let mut buf = Vec::new();
        let result = install_hook(&cfg, &git_dir, &mut buf);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not installed by ops"));
    }

    // -- ensure_config_command --

    #[test]
    fn ensure_config_creates_command_in_empty_file() {
        let cfg = commit_config();
        let dir = tempfile::tempdir().expect("tempdir");

        let selected = vec!["verify".to_string()];
        let mut buf = Vec::new();
        ensure_config_command(&cfg, dir.path(), &selected, &mut buf)
            .expect("ensure_config_command");

        let content = std::fs::read_to_string(dir.path().join(".ops.toml")).unwrap();
        assert!(content.contains("[commands.run-before-commit]"));
        assert!(content.contains("verify"));
        assert!(content.contains("fail_fast"));

        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("Added"));
    }

    #[test]
    fn ensure_config_creates_push_command() {
        let cfg = push_config();
        let dir = tempfile::tempdir().expect("tempdir");

        let selected = vec!["verify".to_string()];
        let mut buf = Vec::new();
        ensure_config_command(&cfg, dir.path(), &selected, &mut buf)
            .expect("ensure_config_command");

        let content = std::fs::read_to_string(dir.path().join(".ops.toml")).unwrap();
        assert!(content.contains("[commands.run-before-push]"));
    }

    #[test]
    fn ensure_config_preserves_existing_command() {
        let cfg = commit_config();
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join(".ops.toml"),
            "[commands.run-before-commit]\ncommands = [\"test\"]\n",
        )
        .unwrap();

        let selected = vec!["verify".to_string()];
        let mut buf = Vec::new();
        ensure_config_command(&cfg, dir.path(), &selected, &mut buf)
            .expect("ensure_config_command");

        let content = std::fs::read_to_string(dir.path().join(".ops.toml")).unwrap();
        assert!(content.contains(r#"commands = ["test"]"#));

        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("already defined"));
    }

    #[test]
    fn ensure_config_appends_to_existing_config() {
        let cfg = commit_config();
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join(".ops.toml"),
            "[output]\ntheme = \"compact\"\n\n[commands.build]\nprogram = \"cargo\"\nargs = [\"build\"]\n",
        )
        .unwrap();

        let selected = vec!["build".to_string(), "test".to_string()];
        let mut buf = Vec::new();
        ensure_config_command(&cfg, dir.path(), &selected, &mut buf)
            .expect("ensure_config_command");

        let content = std::fs::read_to_string(dir.path().join(".ops.toml")).unwrap();
        assert!(content.contains("theme = \"compact\""));
        assert!(content.contains("[commands.build]"));
        assert!(content.contains("[commands.run-before-commit]"));
    }

    #[test]
    fn ensure_config_empty_selection_skips() {
        let cfg = commit_config();
        let dir = tempfile::tempdir().expect("tempdir");

        let mut buf = Vec::new();
        ensure_config_command(&cfg, dir.path(), &[], &mut buf).expect("ensure_config_command");

        assert!(!dir.path().join(".ops.toml").exists());

        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("No commands selected"));
    }
}
