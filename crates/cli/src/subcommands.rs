//! Thin wrappers that route each CLI subcommand to its implementation crate.

use std::io::Write;
use std::process::ExitCode;

use anyhow::Context;

#[cfg(feature = "stack-rust")]
use crate::args::ToolsAction;
use crate::args::{
    AboutAction, ExtensionAction, RunBeforeCommitAction, RunBeforePushAction, ThemeAction,
};
#[cfg(feature = "stack-rust")]
use crate::tools_cmd;
use crate::{
    about_cmd, extension_cmd, run_before_commit_cmd, run_before_push_cmd, run_cmd, theme_cmd,
};

pub(crate) fn run_about(refresh: bool, action: Option<AboutAction>) -> anyhow::Result<()> {
    let (config, cwd) = crate::load_config_and_cwd()?;
    let registry = crate::registry::build_data_registry(&config, &cwd)?;
    match action {
        Some(AboutAction::Setup) => about_cmd::run_about_setup(&registry),
        #[cfg(feature = "duckdb")]
        Some(AboutAction::Code) => ops_about::run_about_code(&registry),
        #[cfg(not(feature = "duckdb"))]
        Some(AboutAction::Code) => {
            anyhow::bail!("about code requires the duckdb feature");
        }
        Some(AboutAction::Crates | AboutAction::Modules) => ops_about::run_about_units(&registry),
        Some(AboutAction::Coverage) => ops_about::run_about_coverage(&registry),
        Some(AboutAction::Dependencies) => ops_about::run_about_deps(&registry),
        None => {
            let columns = config.output.columns;
            let opts = ops_about::AboutOptions {
                refresh,
                visible_fields: config.about.fields.clone(),
            };
            ops_about::run_about(&registry, &opts, columns, &cwd, &mut std::io::stdout())
        }
    }
}

#[cfg(feature = "stack-rust")]
pub(crate) fn run_deps(refresh: bool) -> anyhow::Result<()> {
    let (config, cwd) = crate::load_config_and_cwd()?;
    let registry = crate::registry::build_data_registry(&config, &cwd)?;
    let opts = ops_deps::DepsOptions { refresh };
    ops_deps::run_deps(&registry, &opts)
}

pub(crate) fn run_theme(action: ThemeAction) -> anyhow::Result<()> {
    match action {
        ThemeAction::List => theme_cmd::run_theme_list(),
        ThemeAction::Select => theme_cmd::run_theme_select(),
    }
}

pub(crate) fn run_extension(action: ExtensionAction) -> anyhow::Result<()> {
    match action {
        ExtensionAction::List => extension_cmd::run_extension_list(),
        ExtensionAction::Show { name } => extension_cmd::run_extension_show(name.as_deref()),
    }
}

/// Prompt the user to run `ops <hook> install` when the hook command is not configured.
fn prompt_hook_install(hook_name: &str) -> anyhow::Result<ExitCode> {
    let _ = writeln!(
        std::io::stderr(),
        "No '{hook_name}' command configured in .ops.toml."
    );
    if !crate::tty::is_stdout_tty() {
        let _ = writeln!(
            std::io::stderr(),
            "Run `ops {hook_name} install` to set it up."
        );
        return Ok(ExitCode::FAILURE);
    }
    let answer = inquire::Confirm::new(&format!("Run `ops {hook_name} install` now?"))
        .with_default(true)
        .prompt()?;
    if answer {
        match hook_name {
            "run-before-commit" => run_before_commit_cmd::run_before_commit_install()?,
            "run-before-push" => run_before_push_cmd::run_before_push_install()?,
            other => anyhow::bail!("unknown hook: {other}"),
        }
        return Ok(ExitCode::SUCCESS);
    }
    Ok(ExitCode::SUCCESS)
}

/// Static hook descriptor: everything the shared dispatch needs to run a
/// `run-before-{commit,push}` hook without re-implementing the same skip /
/// prompt / dispatch dance per hook.
type HookPreflight = (fn() -> anyhow::Result<bool>, &'static str);

struct HookDispatch {
    name: &'static str,
    skip_env_var: &'static str,
    should_skip: fn() -> bool,
    /// Optional pre-flight predicate; returning `Ok(false)` short-circuits with
    /// the supplied skip message instead of executing the hook command.
    preflight: Option<HookPreflight>,
    install: fn() -> anyhow::Result<()>,
}

const HOOK_BEFORE_COMMIT: HookDispatch = HookDispatch {
    name: "run-before-commit",
    skip_env_var: ops_run_before_commit::SKIP_ENV_VAR,
    should_skip: ops_run_before_commit::should_skip,
    preflight: Some((ops_run_before_commit::has_staged_files, "no staged files")),
    install: run_before_commit_cmd::run_before_commit_install,
};

const HOOK_BEFORE_PUSH: HookDispatch = HookDispatch {
    name: "run-before-push",
    skip_env_var: ops_run_before_push::SKIP_ENV_VAR,
    should_skip: ops_run_before_push::should_skip,
    preflight: None,
    install: run_before_push_cmd::run_before_push_install,
};

fn run_hook_dispatch(hook: &HookDispatch, run_preflight: bool) -> anyhow::Result<ExitCode> {
    let config = ops_core::config::load_config()
        .with_context(|| format!("failed to load config for {} check", hook.name))?;
    if !config.commands.contains_key(hook.name) {
        return prompt_hook_install(hook.name);
    }
    if (hook.should_skip)() {
        let _ = writeln!(
            std::io::stderr(),
            "[{}] {}=1 — skipping",
            hook.name,
            hook.skip_env_var
        );
        return Ok(ExitCode::SUCCESS);
    }
    if run_preflight {
        if let Some((predicate, skip_msg)) = hook.preflight {
            if !predicate()? {
                let _ = writeln!(std::io::stderr(), "[{}] {} — skipping", hook.name, skip_msg);
                return Ok(ExitCode::SUCCESS);
            }
        }
    }
    let args = vec![std::ffi::OsString::from(hook.name)];
    run_cmd::run_external_command(&args, false, false, None, false)
}

pub(crate) fn run_before_commit(
    action: Option<RunBeforeCommitAction>,
    changed_only: bool,
) -> anyhow::Result<ExitCode> {
    match action {
        Some(RunBeforeCommitAction::Install) => {
            (HOOK_BEFORE_COMMIT.install)()?;
            Ok(ExitCode::SUCCESS)
        }
        None => run_hook_dispatch(&HOOK_BEFORE_COMMIT, changed_only),
    }
}

pub(crate) fn run_before_push(
    action: Option<RunBeforePushAction>,
    _changed_only: bool,
) -> anyhow::Result<ExitCode> {
    match action {
        Some(RunBeforePushAction::Install) => {
            (HOOK_BEFORE_PUSH.install)()?;
            Ok(ExitCode::SUCCESS)
        }
        None => run_hook_dispatch(&HOOK_BEFORE_PUSH, false),
    }
}

#[cfg(feature = "stack-rust")]
pub(crate) fn run_tools(action: ToolsAction) -> anyhow::Result<ExitCode> {
    match action {
        ToolsAction::List => {
            tools_cmd::run_tools_list()?;
            Ok(ExitCode::SUCCESS)
        }
        ToolsAction::Check => tools_cmd::run_tools_check(),
        ToolsAction::Install { name } => tools_cmd::run_tools_install(name.as_deref()),
    }
}
