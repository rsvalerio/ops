//! Run-before-push hook extension: install and manage git pre-push hooks.

use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Context;
use ops_extension::ExtensionType;

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

/// Returns `true` if `SKIP_OPS_RUN_BEFORE_PUSH=1` is set.
pub fn should_skip() -> bool {
    std::env::var(SKIP_ENV_VAR).is_ok_and(|v| v == "1")
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

/// Install the git pre-push hook.
///
/// Returns the path to the created hook file.
pub fn install_hook(git_dir: &Path, w: &mut dyn Write) -> anyhow::Result<PathBuf> {
    let hooks_dir = git_dir.join("hooks");
    std::fs::create_dir_all(&hooks_dir).context("failed to create .git/hooks directory")?;

    let hook_path = hooks_dir.join("pre-push");

    if hook_path.exists() {
        let existing =
            std::fs::read_to_string(&hook_path).context("failed to read existing hook")?;
        if existing == HOOK_SCRIPT {
            writeln!(w, "Hook already installed at {}", hook_path.display())?;
            return Ok(hook_path);
        }
        if existing.contains("ops run-before-push") || existing.contains("ops before-push") {
            // Old/outdated ops hook — overwrite it below
            writeln!(w, "Updating outdated ops hook at {}", hook_path.display())?;
        } else {
            anyhow::bail!(
                "a pre-push hook already exists at {} and was not installed by ops. \
                 Remove it manually or back it up before running install.",
                hook_path.display()
            );
        }
    }

    std::fs::write(&hook_path, HOOK_SCRIPT).context("failed to write hook")?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&hook_path, std::fs::Permissions::from_mode(0o755))
            .context("failed to make hook executable")?;
    }

    writeln!(w, "Installed hook at {}", hook_path.display())?;
    Ok(hook_path)
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

    // Check if run-before-push command already exists
    if let Some(commands) = doc.get("commands").and_then(|c| c.as_table()) {
        if commands.contains_key("run-before-push") {
            writeln!(w, "Command 'run-before-push' already defined in .ops.toml")?;
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
    cmd.insert(
        "help",
        toml_edit::value("Run run-before-push checks before pushing"),
    );

    commands.insert("run-before-push", toml_edit::Item::Table(cmd));

    std::fs::write(&config_path, doc.to_string()).context("failed to write .ops.toml")?;
    writeln!(w, "Added 'run-before-push' command to .ops.toml")?;

    Ok(())
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
