//! Shared logic for git hook install commands (run-before-commit, run-before-push).

use std::io::{self, Write};
use std::path::{Path, PathBuf};

use ops_core::config::{CommandSpec, Config};
use ops_core::stack::Stack;

use crate::tty::SelectOption;

/// Hook-specific operations provided by each extension crate.
pub struct HookOps {
    pub hook_name: &'static str,
    pub find_git_dir: fn(&Path) -> Option<PathBuf>,
    pub install_hook: fn(&Path, &mut dyn Write) -> anyhow::Result<PathBuf>,
    pub ensure_config_command: fn(&Path, &[String], &mut dyn Write) -> anyhow::Result<()>,
}

/// Shared interactive install orchestration for all hook types.
///
/// TASK-0427: takes the pre-resolved CLI config so the install path does
/// not re-parse `.ops.toml` after `run()` already loaded it.
pub fn run_hook_install(config: &Config, ops: &HookOps) -> anyhow::Result<()> {
    crate::tty::require_tty(&format!("{} install", ops.hook_name))?;

    let cwd = std::env::current_dir()?;
    let git_dir = (ops.find_git_dir)(&cwd)
        .ok_or_else(|| anyhow::anyhow!("not inside a git repository (no .git found)"))?;

    let stack = Stack::resolve(config.stack.as_deref(), &cwd);

    // ARCH-2 / TASK-0719: build the extension `CommandRegistry` once here
    // (matching the runner-wiring path in `run_cmd::setup_extensions`) and
    // pass it down. Previously `gather_available_commands` re-instantiated
    // every compiled-in extension internally on each call, duplicating
    // factory I/O and re-emitting collision warnings.
    let mut cmd_registry = ops_extension::CommandRegistry::new();
    match crate::registry::builtin_extensions(config, &cwd) {
        Ok(exts) => {
            let ext_refs = crate::registry::as_ext_refs(&exts);
            crate::registry::register_extension_commands(&ext_refs, &mut cmd_registry);
        }
        Err(e) => {
            tracing::warn!(
                error = %format!("{e:#}"),
                "failed to load extensions for hook command selection; extension-provided commands will be omitted",
            );
            ops_core::ui::warn(format!(
                "could not load extensions for {} install: {e:#}\n  extension-provided commands will not appear in the selection list",
                ops.hook_name,
            ));
        }
    }

    let options = gather_available_commands(config, stack, &cmd_registry, ops.hook_name);

    let selected = if options.is_empty() {
        ops_core::ui::note(
            "no commands found. Install the hook anyway; configure commands in .ops.toml later.",
        );
        vec![]
    } else {
        let prompt = format!("Select commands to run in {} hook:", ops.hook_name);
        let selections = inquire::MultiSelect::new(&prompt, options).prompt()?;
        selections.into_iter().map(|o| o.name).collect()
    };

    let mut w = io::stdout();
    (ops.install_hook)(&git_dir, &mut w)?;
    (ops.ensure_config_command)(&cwd, &selected, &mut w)?;

    Ok(())
}

/// Non-interactive install with explicit command list (for testing).
#[cfg(test)]
pub fn run_hook_install_with(
    ops: &HookOps,
    selected_commands: &[String],
    w: &mut dyn Write,
) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let git_dir = (ops.find_git_dir)(&cwd)
        .ok_or_else(|| anyhow::anyhow!("not inside a git repository (no .git found)"))?;

    (ops.install_hook)(&git_dir, w)?;
    (ops.ensure_config_command)(&cwd, selected_commands, w)?;

    Ok(())
}

/// Collect available commands for hook selection, excluding the hook's own command name.
///
/// Sources are checked in priority order: config > stack defaults > extension commands.
/// Later sources are deduped against earlier ones.
///
/// ARCH-2 / TASK-0719: takes a pre-built `CommandRegistry` so the helper
/// stays a synchronous data-shaping function — no factory probes, no
/// extension I/O, no re-emitted collision warnings on repeated calls. The
/// caller (`run_hook_install`) builds the registry once via
/// `register_extension_commands` and passes it down.
pub fn gather_available_commands(
    config: &Config,
    stack: Option<Stack>,
    cmd_registry: &ops_extension::CommandRegistry,
    exclude_name: &str,
) -> Vec<SelectOption> {
    let mut seen = std::collections::HashSet::new();
    let mut options = Vec::new();

    // Config commands first (higher priority)
    for (name, spec) in &config.commands {
        if name == exclude_name {
            continue;
        }
        seen.insert(name.clone());
        options.push(SelectOption {
            name: name.clone(),
            description: command_description(spec),
        });
    }

    // Stack default commands (lower priority, deduped)
    if let Some(stack) = stack {
        for (name, spec) in stack.default_commands() {
            if name == exclude_name || seen.contains(&name) {
                continue;
            }
            seen.insert(name.clone());
            options.push(SelectOption {
                name,
                description: command_description(&spec),
            });
        }
    }

    // Extension commands (lowest priority, deduped)
    for (name, spec) in cmd_registry {
        let name_str = name.to_string();
        if name_str == exclude_name || seen.contains(&name_str) {
            continue;
        }
        seen.insert(name_str.clone());
        options.push(SelectOption {
            name: name_str,
            description: command_description(spec),
        });
    }

    options
}

pub fn command_description(spec: &CommandSpec) -> String {
    spec.help()
        .map(|s| s.to_string())
        .unwrap_or_else(|| spec.display_cmd_fallback())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ops_core::config::{CommandSpec, CompositeCommandSpec, ExecCommandSpec};

    fn config_with_hook_and_build(hook_name: &str) -> Config {
        let mut config = Config::default();
        config.extensions.enabled = Some(vec![]);
        config.commands.insert(
            hook_name.to_string(),
            CommandSpec::Composite(CompositeCommandSpec::new(["verify"])),
        );
        config.commands.insert(
            "build".to_string(),
            CommandSpec::Exec(ExecCommandSpec::new("cargo", ["build"])),
        );
        config
    }

    // -- command_description --

    #[test]
    fn command_description_exec_with_help() {
        let mut exec = ExecCommandSpec::new("cargo", ["build"]);
        exec.help = Some("Build the project".to_string());
        let spec = CommandSpec::Exec(exec);
        assert_eq!(command_description(&spec), "Build the project");
    }

    #[test]
    fn command_description_exec_without_help() {
        let spec = CommandSpec::Exec(ExecCommandSpec::new("cargo", ["build", "--release"]));
        let desc = command_description(&spec);
        assert!(desc.contains("cargo"), "got: {desc}");
        assert!(desc.contains("build"), "got: {desc}");
    }

    #[test]
    fn command_description_composite_with_help() {
        let mut comp = CompositeCommandSpec::new(["build", "test"]);
        comp.help = Some("Build and test".to_string());
        let spec = CommandSpec::Composite(comp);
        assert_eq!(command_description(&spec), "Build and test");
    }

    #[test]
    fn command_description_composite_without_help() {
        let spec = CommandSpec::Composite(CompositeCommandSpec::new(["build", "test"]));
        let desc = command_description(&spec);
        // Should fall back to display_cmd_fallback which shows the composite commands
        assert!(!desc.is_empty(), "description should not be empty");
    }

    #[test]
    fn gather_excludes_hook_command() {
        let empty = ops_extension::CommandRegistry::new();
        for hook_name in ["run-before-commit", "run-before-push"] {
            let config = config_with_hook_and_build(hook_name);
            let options = gather_available_commands(&config, None, &empty, hook_name);
            assert_eq!(options.len(), 1, "hook={hook_name}");
            assert_eq!(options[0].name, "build", "hook={hook_name}");
        }
    }

    #[test]
    fn gather_merges_config_and_stack() {
        let mut config = Config::default();
        config.commands.insert(
            "lint".to_string(),
            CommandSpec::Exec(ExecCommandSpec::new("eslint", Vec::<String>::new())),
        );

        let empty = ops_extension::CommandRegistry::new();
        let options =
            gather_available_commands(&config, Some(Stack::Rust), &empty, "run-before-commit");
        let names: Vec<&str> = options.iter().map(|o| o.name.as_str()).collect();

        assert!(names.contains(&"lint"));
        assert!(names.contains(&"build"));
        assert!(names.contains(&"test"));
        assert!(names.contains(&"verify"));
    }

    /// ARCH-2 / TASK-0719: gather_available_commands is now a pure data
    /// reshape — it accepts a pre-built CommandRegistry, so callers can
    /// inject a mock registry without compiling-in any extension state.
    #[test]
    fn gather_includes_injected_registry_commands() {
        let config = Config::default();
        let mut registry = ops_extension::CommandRegistry::new();
        registry.insert(
            "deploy".into(),
            CommandSpec::Exec(ExecCommandSpec::new("deploy.sh", Vec::<String>::new())),
        );

        let options = gather_available_commands(&config, None, &registry, "run-before-commit");
        let names: Vec<&str> = options.iter().map(|o| o.name.as_str()).collect();
        assert!(
            names.contains(&"deploy"),
            "extension-provided commands should appear in the selection list: {names:?}"
        );
    }

    #[test]
    fn gather_config_takes_priority_over_stack() {
        let mut config = Config::default();
        config.commands.insert(
            "build".to_string(),
            CommandSpec::Exec({
                let mut spec = ExecCommandSpec::new("make", Vec::<String>::new());
                spec.help = Some("Custom build".to_string());
                spec
            }),
        );

        let empty = ops_extension::CommandRegistry::new();
        let options =
            gather_available_commands(&config, Some(Stack::Rust), &empty, "run-before-commit");
        let build = options.iter().find(|o| o.name == "build").unwrap();
        assert!(build.description.contains("Custom build"));
    }
}
