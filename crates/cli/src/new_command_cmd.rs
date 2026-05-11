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

    let (program, args) = parse_command(full_command)?;

    let name = inquire::Text::new("Command name:")
        .with_help_message(
            "used in [commands.<name>] and `ops <name>`; \
             no whitespace, control chars, '/' or '\\', and not starting with '-'",
        )
        .with_validator(|input: &str| {
            Ok(match validate_command_name(input.trim()) {
                Ok(()) => inquire::validator::Validation::Valid,
                Err(msg) => inquire::validator::Validation::Invalid(msg.into()),
            })
        })
        .prompt()?;

    let name = name.trim().to_string();
    validate_command_name(&name).map_err(|e| anyhow::anyhow!(e))?;

    append_command_to_config(&name, &program, &args)?;

    writeln!(
        io::stdout(),
        "Added command '{}' to .ops.toml. Run it with: ops {}",
        name,
        name
    )?;
    Ok(())
}

/// Validate that `name` is usable both as a TOML key under `[commands.<name>]`
/// and as a `clap` subcommand name. Empty input, whitespace, control
/// characters, path separators, and a leading `-` (which clap parses as a
/// flag) are all rejected so the on-disk config never ends up with an entry
/// the rest of the tool cannot reach.
fn validate_command_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("command name cannot be empty".into());
    }
    if name.starts_with('-') {
        return Err("command name cannot start with '-' (clap would parse it as a flag)".into());
    }
    for c in name.chars() {
        if c.is_ascii_whitespace() {
            return Err("command name cannot contain whitespace".into());
        }
        if c.is_control() {
            return Err("command name cannot contain control characters".into());
        }
        if c == '/' || c == '\\' {
            return Err("command name cannot contain path separators ('/' or '\\\\')".into());
        }
    }
    Ok(())
}

/// Parse a full command string into (program, args), honouring shell
/// quoting and backslash escapes via [`shlex`]. Unbalanced quotes produce a
/// clear error rather than silently mangling the args.
fn parse_command(input: &str) -> anyhow::Result<(String, Vec<String>)> {
    let parts = shlex::split(input).ok_or_else(|| {
        anyhow::anyhow!(
            "could not parse command (unbalanced quotes or trailing backslash): {input}"
        )
    })?;
    let mut iter = parts.into_iter();
    let program = iter
        .next()
        .ok_or_else(|| anyhow::anyhow!("command cannot be empty"))?;
    let args: Vec<String> = iter.collect();
    Ok((program, args))
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
        let (prog, args) = parse_command("cargo build").unwrap();
        assert_eq!(prog, "cargo");
        assert_eq!(args, vec!["build"]);
    }

    #[test]
    fn parse_command_with_flags() {
        let (prog, args) =
            parse_command("cargo install --path crates/cli --force --all-features").unwrap();
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
        let (prog, args) = parse_command("make").unwrap();
        assert_eq!(prog, "make");
        assert!(args.is_empty());
    }

    #[test]
    fn parse_command_extra_whitespace() {
        let (prog, args) = parse_command("  cargo   test  --lib  ").unwrap();
        assert_eq!(prog, "cargo");
        assert_eq!(args, vec!["test", "--lib"]);
    }

    #[test]
    fn parse_command_quoted_args_preserved() {
        let (prog, args) = parse_command(r#"cargo install --features "a b""#).unwrap();
        assert_eq!(prog, "cargo");
        assert_eq!(args, vec!["install", "--features", "a b"]);
    }

    #[test]
    fn parse_command_escaped_quotes_inside_quotes() {
        let (prog, args) = parse_command(r#"echo "a \"quoted\" word""#).unwrap();
        assert_eq!(prog, "echo");
        assert_eq!(args, vec![r#"a "quoted" word"#]);
    }

    #[test]
    fn parse_command_unbalanced_quote_errors() {
        let result = parse_command(r#"cargo install --features "a b"#);
        assert!(result.is_err(), "expected unbalanced-quote error");
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("could not parse command"),
            "unhelpful error: {msg}"
        );
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
    fn validate_command_name_accepts_typical_names() {
        for name in ["build", "test-suite", "deploy_prod", "ci.smoke", "v2"] {
            assert!(
                validate_command_name(name).is_ok(),
                "expected '{name}' to be accepted"
            );
        }
    }

    #[test]
    fn validate_command_name_rejects_each_pattern_and_skips_write() {
        let cases: &[(&str, &str)] = &[
            ("", "empty"),
            ("build test", "whitespace"),
            ("build\ttest", "tab"),
            ("build\nrelease", "newline"),
            ("bell\x07", "control character"),
            ("--build", "leading dash"),
            ("-x", "leading dash short"),
            ("../escape", "path separator '/'"),
            ("a\\b", "path separator '\\\\'"),
        ];
        for (bad, label) in cases {
            let dir = tempfile::tempdir().expect("tempdir");
            let _guard = crate::CwdGuard::new(dir.path()).expect("CwdGuard");

            assert!(
                validate_command_name(bad).is_err(),
                "expected '{bad}' ({label}) to be rejected"
            );
            assert!(
                !dir.path().join(".ops.toml").exists(),
                "no .ops.toml should be written for rejected name '{bad}' ({label})",
            );
        }
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
