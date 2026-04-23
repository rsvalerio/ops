//! `ops new-command` — interactively add a command to `.ops.toml`.

use std::io::{self, Write};
use std::path::PathBuf;

use anyhow::Context;
use ops_core::config::edit_ops_toml;

pub fn run_new_command() -> anyhow::Result<()> {
    run_new_command_with_tty_check(crate::tty::is_stdout_tty)
}

fn run_new_command_with_tty_check<F>(is_tty: F) -> anyhow::Result<()>
where
    F: FnOnce() -> bool,
{
    crate::tty::require_tty_with("new-command", is_tty)?;

    let full_command = inquire::Text::new("Full command:")
        .with_help_message("e.g. cargo install --path crates/cli --force --all-features")
        .prompt()?;

    let full_command = full_command.trim();
    if full_command.is_empty() {
        anyhow::bail!("command cannot be empty");
    }

    let (program, args) = parse_command(full_command);

    let name = inquire::Text::new("Command name:")
        .with_help_message("the name used in [commands.<name>] and `ops <name>`")
        .prompt()?;

    let name = name.trim().to_string();
    if name.is_empty() {
        anyhow::bail!("command name cannot be empty");
    }

    append_command_to_config(&name, &program, &args)?;

    writeln!(
        io::stdout(),
        "Added command '{}' to .ops.toml. Run it with: ops {}",
        name,
        name
    )?;
    Ok(())
}

/// Parse a full command string into (program, args).
fn parse_command(input: &str) -> (String, Vec<String>) {
    let parts: Vec<&str> = input.split_whitespace().collect();
    let program = parts.first().map(|s| s.to_string()).unwrap_or_default();
    let args: Vec<String> = parts.iter().skip(1).map(|s| s.to_string()).collect();
    (program, args)
}

/// Append a new command entry to `.ops.toml`, creating the file if needed.
fn append_command_to_config(name: &str, program: &str, args: &[String]) -> anyhow::Result<()> {
    let config_path = PathBuf::from(".ops.toml");
    edit_ops_toml(&config_path, |doc| {
        if !doc.contains_key("commands") {
            doc["commands"] = toml_edit::Item::Table(toml_edit::Table::new());
        }
        let commands = doc["commands"]
            .as_table_mut()
            .context("commands is not a table")?;

        if commands.contains_key(name) {
            anyhow::bail!(
                "command '{}' already exists in .ops.toml. Edit it manually or remove it first.",
                name
            );
        }

        let mut cmd = toml_edit::Table::new();
        cmd.insert("program", toml_edit::value(program));
        if !args.is_empty() {
            let mut arr = toml_edit::Array::new();
            for arg in args {
                arr.push(arg.as_str());
            }
            cmd.insert("args", toml_edit::value(arr));
        }
        commands.insert(name, toml_edit::Item::Table(cmd));
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_command_simple() {
        let (prog, args) = parse_command("cargo build");
        assert_eq!(prog, "cargo");
        assert_eq!(args, vec!["build"]);
    }

    #[test]
    fn parse_command_with_flags() {
        let (prog, args) = parse_command("cargo install --path crates/cli --force --all-features");
        assert_eq!(prog, "cargo");
        assert_eq!(
            args,
            vec![
                "install",
                "--path",
                "crates/cli",
                "--force",
                "--all-features"
            ]
        );
    }

    #[test]
    fn parse_command_single_word() {
        let (prog, args) = parse_command("make");
        assert_eq!(prog, "make");
        assert!(args.is_empty());
    }

    #[test]
    fn parse_command_extra_whitespace() {
        let (prog, args) = parse_command("  cargo   test  --lib  ");
        assert_eq!(prog, "cargo");
        assert_eq!(args, vec!["test", "--lib"]);
    }

    #[test]
    fn append_command_creates_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let _guard = crate::CwdGuard::new(dir.path()).expect("CwdGuard");

        append_command_to_config("build", "cargo", &["build".into(), "--release".into()])
            .expect("append");

        let content = std::fs::read_to_string(dir.path().join(".ops.toml")).unwrap();
        assert!(content.contains("[commands.build]"));
        assert!(content.contains(r#"program = "cargo""#));
        assert!(content.contains(r#""build""#));
        assert!(content.contains(r#""--release""#));
    }

    #[test]
    fn append_command_preserves_existing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let _guard = crate::CwdGuard::new(dir.path()).expect("CwdGuard");

        std::fs::write(
            dir.path().join(".ops.toml"),
            r#"[output]
theme = "classic"
"#,
        )
        .unwrap();

        append_command_to_config("test", "cargo", &["test".into()]).expect("append");

        let content = std::fs::read_to_string(dir.path().join(".ops.toml")).unwrap();
        assert!(content.contains(r#"theme = "classic""#));
        assert!(content.contains("[commands.test]"));
        assert!(content.contains(r#"program = "cargo""#));
    }

    #[test]
    fn append_command_rejects_duplicate() {
        let dir = tempfile::tempdir().expect("tempdir");
        let _guard = crate::CwdGuard::new(dir.path()).expect("CwdGuard");

        append_command_to_config("build", "cargo", &["build".into()]).expect("first append");
        let result = append_command_to_config("build", "make", &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn append_command_no_args() {
        let dir = tempfile::tempdir().expect("tempdir");
        let _guard = crate::CwdGuard::new(dir.path()).expect("CwdGuard");

        append_command_to_config("lint", "make", &[]).expect("append");

        let content = std::fs::read_to_string(dir.path().join(".ops.toml")).unwrap();
        assert!(content.contains("[commands.lint]"));
        assert!(content.contains(r#"program = "make""#));
        assert!(!content.contains("args"));
    }

    #[test]
    fn append_command_refuses_to_overwrite_malformed_toml() {
        let dir = tempfile::tempdir().expect("tempdir");
        let _guard = crate::CwdGuard::new(dir.path()).expect("CwdGuard");
        let path = dir.path().join(".ops.toml");
        let malformed = "not = = valid\n{{{";
        std::fs::write(&path, malformed).unwrap();

        let result = append_command_to_config("build", "cargo", &["build".into()]);
        assert!(result.is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), malformed);
    }

    #[test]
    fn new_command_non_tty_returns_error() {
        let result = run_new_command_with_tty_check(|| false);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("interactive terminal"));
    }
}
