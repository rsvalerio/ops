//! CLI handler for `ops pre-commit` subcommands.

use std::io::{self, IsTerminal, Write};

use ops_core::config::{CommandSpec, Config};
use ops_core::stack::Stack;
use ops_pre_commit::{ensure_config_command, find_git_dir, install_hook};

struct CommandOption {
    name: String,
    description: String,
}

impl std::fmt::Display for CommandOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} — {}", self.name, self.description)
    }
}

fn gather_available_commands(
    config: &Config,
    stack: Option<Stack>,
    cwd: &std::path::Path,
) -> Vec<CommandOption> {
    let mut seen = std::collections::HashSet::new();
    let mut options = Vec::new();

    // Config commands first (higher priority)
    for (name, spec) in &config.commands {
        if name == "pre-commit" {
            continue;
        }
        seen.insert(name.clone());
        options.push(CommandOption {
            name: name.clone(),
            description: command_description(spec),
        });
    }

    // Stack default commands (lower priority, deduped)
    if let Some(stack) = stack {
        for (name, spec) in stack.default_commands() {
            if name == "pre-commit" || seen.contains(&name) {
                continue;
            }
            seen.insert(name.clone());
            options.push(CommandOption {
                name,
                description: command_description(&spec),
            });
        }
    }

    // Extension commands (lowest priority, deduped)
    if let Ok(exts) = crate::registry::builtin_extensions(config, cwd) {
        let ext_refs = crate::registry::as_ext_refs(&exts);
        let mut cmd_registry = ops_extension::CommandRegistry::new();
        crate::registry::register_extension_commands(&ext_refs, &mut cmd_registry);
        for (name, spec) in &cmd_registry {
            if name == "pre-commit" || seen.contains(name) {
                continue;
            }
            seen.insert(name.clone());
            options.push(CommandOption {
                name: name.clone(),
                description: command_description(spec),
            });
        }
    }

    options
}

fn command_description(spec: &CommandSpec) -> String {
    spec.help()
        .map(|s| s.to_string())
        .unwrap_or_else(|| spec.display_cmd_fallback())
}

pub fn run_pre_commit_install() -> anyhow::Result<()> {
    if !io::stdout().is_terminal() {
        anyhow::bail!("pre-commit install requires an interactive terminal");
    }

    let cwd = std::env::current_dir()?;
    let git_dir = find_git_dir(&cwd)
        .ok_or_else(|| anyhow::anyhow!("not inside a git repository (no .git found)"))?;

    let config = ops_core::config::load_config().unwrap_or_default();
    let stack = Stack::resolve(config.stack.as_deref(), &cwd);

    let options = gather_available_commands(&config, stack, &cwd);

    let selected = if options.is_empty() {
        writeln!(
            io::stderr(),
            "No commands found. Install the hook anyway; configure commands in .ops.toml later."
        )?;
        vec![]
    } else {
        let selections =
            inquire::MultiSelect::new("Select commands to run in pre-commit hook:", options)
                .prompt()?;
        selections.into_iter().map(|o| o.name).collect()
    };

    let mut w = io::stdout();
    install_hook(&git_dir, &mut w)?;
    ensure_config_command(&cwd, &selected, &mut w)?;

    Ok(())
}

/// Non-interactive install with explicit command list (for testing).
#[cfg(test)]
fn run_pre_commit_install_with(
    selected_commands: &[String],
    w: &mut dyn Write,
) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let git_dir = find_git_dir(&cwd)
        .ok_or_else(|| anyhow::anyhow!("not inside a git repository (no .git found)"))?;

    install_hook(&git_dir, w)?;
    ensure_config_command(&cwd, selected_commands, w)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CwdGuard;

    #[test]
    fn install_creates_hook_and_config() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(dir.path().join(".git")).unwrap();
        let _guard = CwdGuard::new(dir.path()).expect("CwdGuard");

        let selected = vec!["verify".to_string()];
        let mut buf = Vec::new();
        run_pre_commit_install_with(&selected, &mut buf).expect("install");

        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("Installed hook"));
        assert!(output.contains("Added"));

        // Hook exists and is executable
        let hook_path = dir.path().join(".git/hooks/pre-commit");
        assert!(hook_path.exists());

        // Config exists with pre-commit command
        let config = std::fs::read_to_string(dir.path().join(".ops.toml")).unwrap();
        assert!(config.contains("[commands.pre-commit]"));
        assert!(config.contains("verify"));
    }

    #[test]
    fn install_with_empty_selection_skips_config() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(dir.path().join(".git")).unwrap();
        let _guard = CwdGuard::new(dir.path()).expect("CwdGuard");

        let mut buf = Vec::new();
        run_pre_commit_install_with(&[], &mut buf).expect("install");

        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("Installed hook"));
        assert!(output.contains("No commands selected"));

        // Hook exists but no config
        assert!(dir.path().join(".git/hooks/pre-commit").exists());
        assert!(!dir.path().join(".ops.toml").exists());
    }

    #[test]
    fn install_no_git_dir_errors() {
        let dir = tempfile::tempdir().expect("tempdir");
        let _guard = CwdGuard::new(dir.path()).expect("CwdGuard");

        let mut buf = Vec::new();
        let result = run_pre_commit_install_with(&[], &mut buf);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not inside a git"));
    }

    #[test]
    fn gather_excludes_pre_commit() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut config = Config::default();
        config.extensions.enabled = Some(vec![]);
        config.commands.insert(
            "pre-commit".to_string(),
            CommandSpec::Composite(ops_core::config::CompositeCommandSpec {
                commands: vec!["verify".to_string()],
                parallel: false,
                fail_fast: true,
                help: None,
                aliases: Vec::new(),
            }),
        );
        config.commands.insert(
            "build".to_string(),
            CommandSpec::Exec(ops_core::config::ExecCommandSpec {
                program: "cargo".to_string(),
                args: vec!["build".to_string()],
                ..Default::default()
            }),
        );

        let options = gather_available_commands(&config, None, dir.path());
        assert_eq!(options.len(), 1);
        assert_eq!(options[0].name, "build");
    }

    #[test]
    fn gather_merges_config_and_stack() {
        let mut config = Config::default();
        config.commands.insert(
            "lint".to_string(),
            CommandSpec::Exec(ops_core::config::ExecCommandSpec {
                program: "eslint".to_string(),
                ..Default::default()
            }),
        );

        let options =
            gather_available_commands(&config, Some(Stack::Rust), std::path::Path::new("."));
        let names: Vec<&str> = options.iter().map(|o| o.name.as_str()).collect();

        // Config command present
        assert!(names.contains(&"lint"));
        // Stack defaults present
        assert!(names.contains(&"build"));
        assert!(names.contains(&"test"));
        assert!(names.contains(&"verify"));
    }

    #[test]
    fn gather_config_takes_priority_over_stack() {
        let mut config = Config::default();
        config.commands.insert(
            "build".to_string(),
            CommandSpec::Exec(ops_core::config::ExecCommandSpec {
                program: "make".to_string(),
                help: Some("Custom build".to_string()),
                ..Default::default()
            }),
        );

        let options =
            gather_available_commands(&config, Some(Stack::Rust), std::path::Path::new("."));
        let build = options.iter().find(|o| o.name == "build").unwrap();
        assert!(build.description.contains("Custom build"));
    }
}
