//! Thin wrappers that route each CLI subcommand to its implementation crate.

use std::path::PathBuf;
use std::process::ExitCode;

use ops_core::config::Config;

#[cfg(feature = "stack-rust")]
use crate::args::ToolsAction;
use crate::args::{
    AboutAction, ExtensionAction, RunBeforeCommitAction, RunBeforePushAction, ThemeAction,
};
#[cfg(feature = "stack-rust")]
use crate::tools_cmd;
use crate::{about_cmd, extension_cmd, pre_hook_cmd, run_cmd, theme_cmd};

/// Shared cwd + registry preamble used by `run_about`, `run_deps`, and the
/// extension subcommand handlers. DUP-1 / TASK-0207 collapsed the original
/// per-handler boilerplate; TASK-0427 then threaded the pre-resolved
/// `Config` so the helper no longer re-loads `.ops.toml`.
pub(crate) fn cli_data_context(
    config: &Config,
) -> anyhow::Result<(PathBuf, ops_extension::DataRegistry)> {
    let cwd = crate::cwd()?;
    let registry = crate::registry::build_data_registry(config, &cwd)?;
    Ok((cwd, registry))
}

pub(crate) fn run_about(
    config: &Config,
    refresh: bool,
    action: Option<AboutAction>,
) -> anyhow::Result<()> {
    let (cwd, registry) = cli_data_context(config)?;
    match action {
        Some(AboutAction::Setup) => about_cmd::run_about_setup(config, &registry, &cwd),
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
            // PERF-1 / TASK-0895: removed the misleading `from_ref` wrapper
            // that pretended to avoid cloning while still calling `to_vec`.
            // A direct `clone()` on the (typically small) config Vec is
            // honest about the allocation cost.
            let opts = ops_about::AboutOptions::new(
                refresh,
                config.about.fields.clone(),
                crate::tty::is_stdout_tty(),
            );
            ops_about::run_about(&registry, &opts, &cwd, &mut std::io::stdout())
        }
    }
}

#[cfg(feature = "stack-rust")]
pub(crate) fn run_deps(config: &Config, refresh: bool) -> anyhow::Result<()> {
    let (_cwd, registry) = cli_data_context(config)?;
    let opts = ops_deps::DepsOptions { refresh };
    ops_deps::run_deps(&registry, &opts)
}

pub(crate) fn run_theme(config: &Config, action: ThemeAction) -> anyhow::Result<()> {
    match action {
        ThemeAction::List => theme_cmd::run_theme_list(config),
        ThemeAction::Select => theme_cmd::run_theme_select(config),
    }
}

pub(crate) fn run_extension(config: &Config, action: ExtensionAction) -> anyhow::Result<()> {
    match action {
        ExtensionAction::List => extension_cmd::run_extension_list(config),
        ExtensionAction::Show { name } => {
            extension_cmd::run_extension_show(config, name.as_deref())
        }
    }
}

/// Prompt the user to run `ops <hook> install` when the hook command is not configured.
fn prompt_hook_install(config: &Config, hook_name: &str) -> anyhow::Result<ExitCode> {
    ops_core::ui::note(format!("no '{hook_name}' command configured in .ops.toml."));
    // Don't block on a TTY prompt under cargo test / CI / scripted
    // invocations: `is_stdout_tty()` is true when `cargo test` inherits
    // the user's terminal, so the test that exercises `run_hook_dispatch`
    // would otherwise hang waiting for `inquire::Confirm`. Honor an
    // explicit non-interactive opt-out (and the conventional `CI` flag)
    // before consulting the TTY.
    let noninteractive =
        std::env::var_os("OPS_NONINTERACTIVE").is_some() || std::env::var_os("CI").is_some();
    if noninteractive || !crate::tty::is_stdout_tty() {
        ops_core::ui::note(format!("run `ops {hook_name} install` to set it up."));
        return Ok(ExitCode::FAILURE);
    }
    let answer = inquire::Confirm::new(&format!("Run `ops {hook_name} install` now?"))
        .with_default(true)
        .prompt()?;
    if answer {
        match hook_name {
            "run-before-commit" => pre_hook_cmd::run_before_commit_install(config)?,
            "run-before-push" => pre_hook_cmd::run_before_push_install(config)?,
            other => anyhow::bail!("unknown hook: {other}"),
        }
        return Ok(ExitCode::SUCCESS);
    }
    Ok(ExitCode::SUCCESS)
}

/// TASK-0757: dispatch consumes the same `HookOps` descriptor that the install
/// path uses, so adding a new hook means editing one constant table in
/// `pre_hook_cmd` rather than two parallel ones.
fn run_hook_dispatch(
    config: std::sync::Arc<Config>,
    hook: &crate::hook_shared::HookOps,
    run_preflight: bool,
) -> anyhow::Result<ExitCode> {
    if !config.commands.contains_key(hook.hook_name) {
        return prompt_hook_install(&config, hook.hook_name);
    }
    if (hook.should_skip)() {
        ops_core::ui::note(format!(
            "[{}] {}=1 — skipping",
            hook.hook_name, hook.skip_env_var
        ));
        return Ok(ExitCode::SUCCESS);
    }
    if run_preflight {
        if let Some((predicate, skip_msg)) = hook.preflight {
            if !predicate()? {
                ops_core::ui::note(format!("[{}] {} — skipping", hook.hook_name, skip_msg));
                return Ok(ExitCode::SUCCESS);
            }
        }
    }
    let args = vec![std::ffi::OsString::from(hook.hook_name)];
    // SEC-14 / TASK-0886: a `.ops.toml` landed by a coworker PR is the
    // documented threat model for the hook path. Refuse to spawn when the
    // spec's cwd escapes the workspace, instead of the interactive
    // `WarnAndAllow` default that only logs.
    run_cmd::run_external_command(
        config,
        &args,
        run_cmd::RunOptions {
            cwd_escape_policy: ops_runner::command::CwdEscapePolicy::Deny,
            ..Default::default()
        },
    )
}

pub(crate) fn run_before_commit(
    config: std::sync::Arc<Config>,
    action: Option<RunBeforeCommitAction>,
    changed_only: bool,
) -> anyhow::Result<ExitCode> {
    match action {
        Some(RunBeforeCommitAction::Install) => {
            pre_hook_cmd::run_before_commit_install(&config)?;
            Ok(ExitCode::SUCCESS)
        }
        None => run_hook_dispatch(config, &pre_hook_cmd::COMMIT_OPS, changed_only),
    }
}

pub(crate) fn run_before_push(
    config: std::sync::Arc<Config>,
    action: Option<RunBeforePushAction>,
    _changed_only: bool,
) -> anyhow::Result<ExitCode> {
    match action {
        Some(RunBeforePushAction::Install) => {
            pre_hook_cmd::run_before_push_install(&config)?;
            Ok(ExitCode::SUCCESS)
        }
        None => run_hook_dispatch(config, &pre_hook_cmd::PUSH_OPS, false),
    }
}

#[cfg(feature = "stack-rust")]
pub(crate) fn run_tools(config: &Config, action: ToolsAction) -> anyhow::Result<ExitCode> {
    match action {
        ToolsAction::List => {
            tools_cmd::run_tools_list(config)?;
            Ok(ExitCode::SUCCESS)
        }
        ToolsAction::Check => tools_cmd::run_tools_check(config),
        ToolsAction::Install { name } => tools_cmd::run_tools_install(config, name.as_deref()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ERR-1 (TASK-0427): a typical `ops <cmd>` flow must load `.ops.toml`
    /// at most once. Previously `run()` loaded it via
    /// `load_config_or_default("early")` and then `load_config_and_cwd` /
    /// `load_config()` re-loaded inside each handler, so a single CLI
    /// invocation hit the parser multiple times with divergent error
    /// policies. This test pins the new contract: handler-side helpers
    /// take `&Config` and never re-invoke `load_config`.
    #[test]
    #[serial_test::serial]
    fn handlers_do_not_reload_config() {
        let (_dir, _guard) = crate::test_utils::with_temp_config(
            r#"
[commands.echo_test]
program = "echo"
args = ["hi"]
"#,
        );

        // Simulate the early `run()` load.
        ops_core::config::reset_load_config_call_count();
        let config = ops_core::config::load_config_or_default("test-early");
        assert_eq!(
            ops_core::config::load_config_call_count(),
            1,
            "early load should be the only load_config call so far"
        );

        // Each handler-side helper that previously re-loaded `.ops.toml`
        // is now expected to consult the threaded `&Config`.
        let _ = cli_data_context(&config).expect("cli_data_context");

        assert_eq!(
            ops_core::config::load_config_call_count(),
            1,
            "cli_data_context must not reload .ops.toml"
        );

        // run_hook_dispatch's config-presence check used to load_config
        // independently; verify the threaded config now drives that path.
        // The test config has no `run-before-commit` command, so this
        // dispatch falls into `prompt_hook_install`. Force the
        // non-interactive bail path so a tty-attached `cargo test` run
        // does not block on `inquire::Confirm`.
        // #[serial] guards this from clobbering other tests.
        std::env::set_var("OPS_NONINTERACTIVE", "1");
        let _ = run_hook_dispatch(
            std::sync::Arc::new(config),
            &pre_hook_cmd::COMMIT_OPS,
            false,
        );
        std::env::remove_var("OPS_NONINTERACTIVE");
        assert_eq!(
            ops_core::config::load_config_call_count(),
            1,
            "run_hook_dispatch must not reload .ops.toml"
        );
    }
}
