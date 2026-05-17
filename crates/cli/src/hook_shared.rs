//! Shared logic for git hook install commands (run-before-commit, run-before-push).

use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Context;
use ops_core::config::{CommandSpec, Config};
use ops_core::stack::Stack;

use crate::tty::SelectOption;
use crate::{ExitCodeOverride, SIGINT_EXIT};

/// Optional pre-flight predicate paired with the message used when it returns
/// `Ok(false)`. Lifted out of `HookOps` to satisfy `clippy::type_complexity`.
pub type HookPreflight = (fn() -> anyhow::Result<bool>, &'static str);

/// Hook-specific operations provided by each extension crate.
///
/// Collapses the parallel `HookDispatch` descriptor that previously lived in
/// `subcommands` into this single struct so adding a new hook means editing
/// one constant table, not two.
pub struct HookOps {
    pub hook_name: &'static str,
    pub find_git_dir: fn(&Path) -> Option<PathBuf>,
    pub install_hook: fn(&Path, &mut dyn Write) -> anyhow::Result<PathBuf>,
    pub ensure_config_command: fn(&Path, &[String], &mut dyn Write) -> anyhow::Result<()>,
    /// Install entry point used by `Action::Install`. Folds the two
    /// `run_before_*_install` thin wrappers into this field so a single
    /// dispatch helper handles both hooks.
    pub install_fn: fn(&Config) -> anyhow::Result<()>,
    /// Env var that, when set, instructs the dispatcher to skip the hook.
    pub skip_env_var: &'static str,
    /// Returns `true` when the hook should be skipped (e.g. `skip_env_var` set).
    pub should_skip: fn() -> bool,
    /// Optional pre-flight predicate. If `Some`, returning `Ok(false)` short-circuits
    /// the hook with the supplied skip message instead of executing the command.
    pub preflight: Option<HookPreflight>,
}

/// Source of the command list to install in the hook config.
///
/// Collapses the previous split between the production `run_hook_install`
/// (hard-coded `io::stdout()` + `inquire::MultiSelect`) and the test-only
/// `run_hook_install_with` (capturing buffer + fixed list) into one
/// orchestration with a single selection-source seam.
pub enum CommandSelector<'a> {
    /// Production path: present an `inquire::MultiSelect` of available
    /// commands. Requires a TTY.
    Interactive,
    /// Bypass the prompt and use these names verbatim. Only constructed
    /// from tests today, but the variant is part of the public seam so
    /// future scripted callers (a `--commands=…` non-interactive install
    /// flag, for instance) can land without a second entry point.
    #[allow(dead_code)]
    Fixed(&'a [String]),
}

/// Extension-load degradation policy lifted out of `run_hook_install` so
/// the orchestration body covers only the install flow. Hard error if the
/// user opted in to extensions explicitly
/// (`config.extensions.enabled = Some(_)`); soft UI warn otherwise.
fn load_hook_extensions(
    config: &Config,
    cwd: &Path,
    hook_name: &str,
) -> anyhow::Result<ops_extension::CommandRegistry> {
    let mut cmd_registry = ops_extension::CommandRegistry::new();
    match crate::registry::builtin_extensions(config, cwd) {
        Ok(exts) => {
            let ext_refs = crate::registry::as_ext_refs(&exts);
            crate::registry::register_extension_commands(&ext_refs, &mut cmd_registry);
            Ok(cmd_registry)
        }
        Err(e) => {
            // Surface the load failure through exactly one operator-facing
            // channel (UI) — earlier this path double-emitted via
            // `tracing::warn!` *and* `ops_core::ui::warn`.
            if config.extensions.enabled.is_some() {
                anyhow::bail!("could not load extensions for {hook_name} install: {e:#}");
            }
            ops_core::ui::warn(format!(
                "could not load extensions for {hook_name} install: {e:#}\n  extension-provided commands will not appear in the selection list",
            ));
            Ok(cmd_registry)
        }
    }
}

/// Shared install orchestration for all hook types.
///
/// Takes the pre-resolved CLI config so the install path does not re-parse
/// `.ops.toml` after `run()` already loaded it.
///
/// Takes `w: &mut dyn Write` so the production happy-path messages are
/// observable from tests, and a `CommandSelector` so the same entry point
/// covers both interactive and scripted callers.
pub fn run_hook_install(
    config: &Config,
    ops: &HookOps,
    selector: CommandSelector<'_>,
    w: &mut dyn Write,
) -> anyhow::Result<()> {
    // Surface which path operation failed when the process cannot resolve
    // its cwd (EACCES, ENOENT on a deleted dir).
    let cwd = std::env::current_dir().with_context(|| {
        format!(
            "could not determine working directory while installing {} hook",
            ops.hook_name
        )
    })?;
    let git_dir = (ops.find_git_dir)(&cwd)
        .ok_or_else(|| anyhow::anyhow!("not inside a git repository (no .git found)"))?;

    let selected = match selector {
        CommandSelector::Fixed(s) => s.to_vec(),
        CommandSelector::Interactive => {
            crate::tty::require_tty(&format!("{} install", ops.hook_name))?;
            let stack = Stack::resolve(config.stack.as_deref(), &cwd);
            // Build the extension registry once here so
            // `gather_available_commands` stays a pure data-shaper.
            let cmd_registry = load_hook_extensions(config, &cwd, ops.hook_name)?;
            let options = gather_available_commands(config, stack, &cmd_registry, ops.hook_name);

            if options.is_empty() {
                ops_core::ui::note(
                    "no commands found. Install the hook anyway; configure commands in .ops.toml later.",
                );
                Vec::new()
            } else {
                let prompt = format!("Select commands to run in {} hook:", ops.hook_name);
                match inquire::MultiSelect::new(&prompt, options).prompt() {
                    Ok(selections) => selections.into_iter().map(|o| o.name).collect(),
                    // Ctrl-C / Esc at the selection prompt is the user
                    // cancelling — bubble a SIGINT exit via the
                    // `ExitCodeOverride` sentinel instead of a generic
                    // anyhow chain.
                    Err(
                        inquire::InquireError::OperationCanceled
                        | inquire::InquireError::OperationInterrupted,
                    ) => {
                        return Err(anyhow::anyhow!("{} install cancelled", ops.hook_name)
                            .context(ExitCodeOverride(SIGINT_EXIT)));
                    }
                    Err(e) => {
                        return Err(anyhow::Error::new(e)).with_context(|| {
                            format!(
                                "command selection prompt for {} install failed",
                                ops.hook_name
                            )
                        });
                    }
                }
            }
        }
    };

    (ops.install_hook)(&git_dir, w)?;
    (ops.ensure_config_command)(&cwd, &selected, w)?;

    Ok(())
}

/// Collect available commands for hook selection, excluding the hook's own command name.
///
/// Sources are checked in priority order: config > stack defaults > extension commands.
/// Later sources are deduped against earlier ones.
///
/// Takes a pre-built `CommandRegistry` so the helper stays a synchronous
/// data-shaping function — no factory probes, no extension I/O, no
/// re-emitted collision warnings on repeated calls. The caller
/// (`run_hook_install`) builds the registry once via
/// `register_extension_commands` and passes it down.
pub fn gather_available_commands(
    config: &Config,
    stack: Option<Stack>,
    cmd_registry: &ops_extension::CommandRegistry,
    exclude_name: &str,
) -> Vec<SelectOption> {
    let mut options: Vec<SelectOption> = Vec::new();

    // A single helper handles the exclude/dedup/push body so the priority
    // order (config > stack > extensions) is explicit at the call site.
    //
    // OWN-8 (TASK-1358): `try_push` borrows the name and clones only on
    // the surviving-insert branch. Surviving names allocate exactly once
    // (into the `SelectOption`); names that match `exclude_name` or
    // collide with an already-collected option never allocate. Dedup is a
    // linear scan over `options`, which is fine for the handful of
    // commands a hook selection lists.
    let mut try_push = |name: &str, spec: &CommandSpec| {
        if name == exclude_name || options.iter().any(|o| o.name == name) {
            return;
        }
        let description = command_description(spec);
        options.push(SelectOption {
            name: name.to_string(),
            description,
        });
    };

    for (name, spec) in &config.commands {
        try_push(name, spec);
    }
    if let Some(stack) = stack {
        for (name, spec) in stack.default_commands_ref() {
            try_push(name, spec);
        }
    }
    for (name, spec) in cmd_registry {
        try_push(name.as_ref(), spec);
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

    /// gather_available_commands is a pure data reshape — it accepts a
    /// pre-built CommandRegistry, so callers can inject a mock registry
    /// without compiling-in any extension state.
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

    /// OWN-8 (TASK-1358): excluded and duplicate names must not be
    /// pushed into `options`. Asserting *zero allocation* on the reject
    /// path would require a custom global allocator and is not worth the
    /// fixture cost; this test pins the observable behaviour instead —
    /// the hook name is never offered for selection and the same name
    /// appearing in multiple sources surfaces exactly once.
    #[test]
    fn gather_skips_excluded_and_duplicate_names() {
        let mut config = Config::default();
        config.commands.insert(
            "run-before-commit".to_string(),
            CommandSpec::Composite(CompositeCommandSpec::new(["verify"])),
        );
        config.commands.insert(
            "build".to_string(),
            CommandSpec::Exec(ExecCommandSpec::new("cargo", ["build"])),
        );

        let empty = ops_extension::CommandRegistry::new();
        let options =
            gather_available_commands(&config, Some(Stack::Rust), &empty, "run-before-commit");
        let names: Vec<&str> = options.iter().map(|o| o.name.as_str()).collect();

        assert!(
            !names.contains(&"run-before-commit"),
            "excluded hook command leaked into options: {names:?}"
        );
        let build_count = names.iter().filter(|n| **n == "build").count();
        assert_eq!(
            build_count, 1,
            "config 'build' and stack 'build' must dedupe to a single entry: {names:?}"
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
