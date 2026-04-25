//! `.ops.toml` mutation for hook extensions.
//!
//! Adds a `[commands.<name>]` composite entry so the hook script has a
//! meaningful command to dispatch. Uses `toml_edit` to preserve existing
//! formatting and ops-core's atomic write to survive mid-write crashes.

use std::io::Write;
use std::path::Path;

use anyhow::Context;

use crate::HookConfig;

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

    // Read with parse-error propagation (ERR-5) and NotFound-as-empty (SEC-25).
    let mut doc = ops_core::config::read_ops_toml(&config_path)?;

    if let Some(commands) = doc.get("commands").and_then(|c| c.as_table()) {
        if commands.contains_key(config.name) {
            writeln!(w, "Command '{}' already defined in .ops.toml", config.name)?;
            return Ok(());
        }
    }

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

    // Atomic write (SEC-32): sibling temp file + rename so a crash mid-write
    // leaves the user's original .ops.toml intact.
    ops_core::config::write_ops_toml(&config_path, &doc)?;
    writeln!(w, "Added '{}' command to .ops.toml", config.name)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::{commit_config, push_config};

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
    fn ensure_config_refuses_to_overwrite_malformed_toml() {
        // ERR-5 / SEC-32: a parse error must surface as Err and the user's
        // existing (malformed-but-meaningful) file must not be clobbered.
        let cfg = commit_config();
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join(".ops.toml");
        let malformed = "not = = valid\n{{{";
        std::fs::write(&path, malformed).unwrap();

        let selected = vec!["verify".to_string()];
        let mut buf = Vec::new();
        let result = ensure_config_command(&cfg, dir.path(), &selected, &mut buf);

        assert!(result.is_err(), "malformed TOML should be a hard error");
        let err = format!("{:#}", result.unwrap_err());
        assert!(err.contains("TOML"), "err should mention TOML: {err}");
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            malformed,
            "malformed .ops.toml must not be overwritten"
        );
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
