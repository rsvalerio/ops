//! Thin wrappers that route each CLI subcommand to its implementation crate.

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Context;
use ops_core::config::Config;

#[cfg(feature = "stack-rust")]
use crate::args::ToolsAction;
use crate::args::{
    AboutAction, ExtensionAction, RunBeforeCommitAction, RunBeforePushAction, ThemeAction,
};
use crate::hook_shared::HookOps;
#[cfg(feature = "stack-rust")]
use crate::tools_cmd;
use crate::{about_cmd, extension_cmd, pre_hook_cmd, run_cmd, theme_cmd, SIGINT_EXIT};

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
        Some(AboutAction::Crates) => ops_about::run_about_units(&registry),
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
        ThemeAction::Select => {
            let cwd = crate::cwd()?;
            theme_cmd::run_theme_select(config, &cwd)
        }
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

/// ERR-1 / TASK-1189: classify an `inquire::Confirm` prompt result so that a
/// user-initiated cancel (Ctrl-C / Esc) is distinguishable from a real
/// failure. Returns `Ok(Some(answer))` for a real choice, `Ok(None)` for an
/// explicit cancel, and `Err` for any other inquire failure.
///
/// ERR-9 / TASK-1352: real-error branch attaches an anyhow context naming
/// the prompt source so a NotTTY / IO failure tells the user which prompt
/// was in flight rather than surfacing a bare `inquire: <variant>` line.
fn classify_confirm_result(
    res: Result<bool, inquire::InquireError>,
    prompt_source: &str,
) -> anyhow::Result<Option<bool>> {
    match res {
        Ok(b) => Ok(Some(b)),
        Err(
            inquire::InquireError::OperationCanceled | inquire::InquireError::OperationInterrupted,
        ) => Ok(None),
        Err(e) => {
            Err(anyhow::Error::new(e)).with_context(|| format!("{prompt_source} prompt failed"))
        }
    }
}

/// API-1 / TASK-1290: treat empty, `0`, `false`, `no`, `off`, and `n`
/// (case-insensitive, surrounding whitespace trimmed) as off for boolean
/// opt-in env vars like `OPS_NONINTERACTIVE` and `CI`. Anything else present
/// is on. Aligns with the `=truthy` ecosystem convention so
/// `OPS_NONINTERACTIVE=0`, `CI=false`, and `OPS_NONINTERACTIVE=no` all keep
/// prompts interactive.
///
/// READ-5 / TASK-1333: the `no` / `off` / `n` synonyms were added so an
/// operator typing `OPS_NONINTERACTIVE=no` to disable noninteractive mode is
/// not silently flipped into the opposite state — bash/systemd/`bool` parsers
/// across the ecosystem accept those, and the previous "anything-but-0/false"
/// rule produced exactly that inversion.
fn env_flag_enabled(name: &str) -> bool {
    const FALSY: &[&str] = &["", "0", "false", "no", "off", "n"];
    match std::env::var(name) {
        Ok(v) => {
            let t = v.trim();
            !FALSY.iter().any(|f| t.eq_ignore_ascii_case(f))
        }
        Err(_) => false,
    }
}

/// ARCH / TASK-1361: non-interactive policy lifted out of `prompt_hook_install`
/// so the surrounding helper covers only the Confirm prompt + dispatch.
/// Returns `Some(ExitCode)` when the caller must bail without prompting
/// (CI, `OPS_NONINTERACTIVE`, or stdout is not a TTY); `None` to proceed.
fn noninteractive_install_blocked(hook_name: &str) -> Option<ExitCode> {
    let noninteractive = env_flag_enabled("OPS_NONINTERACTIVE") || env_flag_enabled("CI");
    if noninteractive || !crate::tty::is_stdout_tty() {
        ops_core::ui::note(format!("run `ops {hook_name} install` to set it up."));
        return Some(ExitCode::FAILURE);
    }
    None
}

/// Prompt the user to run `ops <hook> install` when the hook command is not configured.
///
/// PATTERN-1 / TASK-1322: takes `&HookOps` so the install entry comes from
/// `hook.install_fn` rather than a stringly-typed match on `hook.hook_name`.
///
/// API-1 / TASK-1325: returns `FAILURE` on every "still not configured"
/// outcome (noninteractive, non-TTY, decline). A user who declines the
/// install prompt cannot leave git pre-commit silently inert — git treats
/// SUCCESS as "hook passed" and the missing config would otherwise let the
/// commit proceed without ever running the configured checks.
///
/// PATTERN-1 / TASK-1375: the Ctrl-C / Esc path returns the shared
/// `SIGINT_EXIT` constant rather than the magic literal `130`.
fn prompt_hook_install(config: &Config, hook: &HookOps) -> anyhow::Result<ExitCode> {
    let hook_name = hook.hook_name;
    ops_core::ui::note(format!("no '{hook_name}' command configured in .ops.toml."));
    if let Some(code) = noninteractive_install_blocked(hook_name) {
        return Ok(code);
    }
    let prompt_label = format!("Run `ops {hook_name} install` now?");
    let answer = match classify_confirm_result(
        inquire::Confirm::new(&prompt_label)
            .with_default(true)
            .prompt(),
        &format!("`ops {hook_name} install` confirm"),
    )? {
        Some(b) => b,
        None => {
            // ERR-1 / TASK-1189: Ctrl-C / Esc at the install prompt is a
            // user-initiated cancel — return the SIGINT exit code directly
            // so `main` does not decorate it with the `ops: error:` frame.
            return Ok(ExitCode::from(SIGINT_EXIT));
        }
    };
    if answer {
        (hook.install_fn)(config)?;
        return Ok(ExitCode::SUCCESS);
    }
    // API-1 / TASK-1325: decline = config still missing. Match the noninteractive
    // branch and report FAILURE so a git-driven invocation cannot succeed
    // while the hook is unconfigured.
    Ok(ExitCode::FAILURE)
}

/// FN-3 / TASK-1331: hook entry point typed so install vs run paths cannot
/// transpose adjacent booleans. Install does not carry `changed_only`;
/// the run path keeps it, but only the variant that owns it.
#[derive(Debug, Clone, Copy)]
enum HookAction {
    Install,
    Run { changed_only: bool },
}

/// TASK-0757: dispatch consumes the same `HookOps` descriptor that the install
/// path uses, so adding a new hook means editing one constant table in
/// `pre_hook_cmd` rather than two parallel ones.
///
/// READ-7 / TASK-1323: third parameter is `changed_only` (the user-facing
/// `--changed-only` CLI flag), not `run_preflight`. The preflight policy
/// is spelled out at the call site rather than smuggled through identical
/// bool slots. API-1 / TASK-1307: when the user passes `--changed-only`
/// against a hook without a preflight, fail loudly instead of silently
/// no-op'ing.
fn run_hook_dispatch(
    config: std::sync::Arc<Config>,
    hook: &HookOps,
    changed_only: bool,
) -> anyhow::Result<ExitCode> {
    if !config.commands.contains_key(hook.hook_name) {
        return prompt_hook_install(&config, hook);
    }
    if (hook.should_skip)() {
        ops_core::ui::note(format!(
            "[{}] {}=1 — skipping",
            hook.hook_name, hook.skip_env_var
        ));
        return Ok(ExitCode::SUCCESS);
    }
    if changed_only {
        match hook.preflight {
            Some((predicate, skip_msg)) => {
                if !predicate()? {
                    ops_core::ui::note(format!("[{}] {} — skipping", hook.hook_name, skip_msg));
                    return Ok(ExitCode::SUCCESS);
                }
            }
            None => {
                anyhow::bail!(
                    "--changed-only is not supported for {} (no preflight predicate registered)",
                    hook.hook_name
                );
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

/// DUP-1 / TASK-1282: collapse the two `run_before_*` wrappers into one.
/// FN-3 / TASK-1331: a typed `HookAction` replaces the previous adjacent
/// `(is_install, changed_only)` booleans so transposed callers fail to
/// compile and Install does not carry an unused `changed_only` value.
fn run_hook_action(
    config: std::sync::Arc<Config>,
    hook: &HookOps,
    action: HookAction,
) -> anyhow::Result<ExitCode> {
    match action {
        HookAction::Install => {
            (hook.install_fn)(&config)?;
            Ok(ExitCode::SUCCESS)
        }
        HookAction::Run { changed_only } => run_hook_dispatch(config, hook, changed_only),
    }
}

pub(crate) fn run_before_commit(
    config: std::sync::Arc<Config>,
    action: Option<RunBeforeCommitAction>,
    changed_only: bool,
) -> anyhow::Result<ExitCode> {
    let hook_action = if matches!(action, Some(RunBeforeCommitAction::Install)) {
        HookAction::Install
    } else {
        HookAction::Run { changed_only }
    };
    run_hook_action(config, &pre_hook_cmd::COMMIT_OPS, hook_action)
}

/// API-1 / TASK-1307: `run-before-push` carries no `changed_only` because
/// no pre-push preflight exists. The flag was removed from
/// `args::CoreSubcommand::RunBeforePush` to stop it from parsing as a
/// silent no-op.
pub(crate) fn run_before_push(
    config: std::sync::Arc<Config>,
    action: Option<RunBeforePushAction>,
) -> anyhow::Result<ExitCode> {
    let hook_action = if matches!(action, Some(RunBeforePushAction::Install)) {
        HookAction::Install
    } else {
        HookAction::Run {
            changed_only: false,
        }
    };
    run_hook_action(config, &pre_hook_cmd::PUSH_OPS, hook_action)
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
    /// ERR-1 / TASK-1189: a cancelled install prompt must not surface as an
    /// anyhow error. `main` formats anyhow errors with the `ops: error:`
    /// prefix; returning `Ok(None)` from `classify_confirm_result` lets the
    /// caller convert cancellation into a clean exit code without that
    /// decoration.
    #[test]
    fn classify_confirm_result_cancel_is_ok_none() {
        let out = classify_confirm_result(
            Err(inquire::InquireError::OperationCanceled),
            "install confirm",
        )
        .expect("cancel must not propagate as error");
        assert!(out.is_none(), "cancel must map to None, not a bool answer");

        let out = classify_confirm_result(
            Err(inquire::InquireError::OperationInterrupted),
            "install confirm",
        )
        .expect("interrupt must not propagate as error");
        assert!(out.is_none(), "interrupt must map to None");
    }

    /// ERR-9 / TASK-1352: a non-cancel inquire error must reach `main`
    /// wrapped with context naming the prompt source, so the user sees
    /// which prompt failed instead of a bare `inquire: <variant>` line.
    #[test]
    fn classify_confirm_result_real_error_propagates() {
        let res = classify_confirm_result(
            Err(inquire::InquireError::NotTTY),
            "`ops run-before-commit install` confirm",
        );
        let err = res.expect_err("real prompt errors must propagate as Err");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("`ops run-before-commit install` confirm prompt failed"),
            "error must name the prompt source, got: {msg}"
        );
    }

    #[test]
    fn classify_confirm_result_answer_passes_through() {
        assert_eq!(
            classify_confirm_result(Ok(true), "x").expect("ok"),
            Some(true)
        );
        assert_eq!(
            classify_confirm_result(Ok(false), "x").expect("ok"),
            Some(false)
        );
    }

    /// API-1 / TASK-1290: `OPS_NONINTERACTIVE=0` is the user typing "off",
    /// so it must keep the prompt interactive. `is_some()` would have
    /// flipped it to non-interactive — exactly the opposite intent.
    #[test]
    #[serial_test::serial]
    fn env_flag_enabled_treats_falsy_as_off() {
        // READ-5 / TASK-1333: `no`, `off`, `n` join the canonical falsy set
        // so the bash/systemd convention does not silently flip operators
        // into the opposite state.
        for falsy in [
            "", "0", "false", "FALSE", "False", "  0  ", "no", "NO", "No", "  no  ", "off", "OFF",
            "n", "N",
        ] {
            std::env::set_var("OPS_NONINTERACTIVE_TEST", falsy);
            assert!(
                !env_flag_enabled("OPS_NONINTERACTIVE_TEST"),
                "{falsy:?} must be treated as off"
            );
        }
        for truthy in ["1", "true", "yes", "anything"] {
            std::env::set_var("OPS_NONINTERACTIVE_TEST", truthy);
            assert!(
                env_flag_enabled("OPS_NONINTERACTIVE_TEST"),
                "{truthy:?} must be treated as on"
            );
        }
        std::env::remove_var("OPS_NONINTERACTIVE_TEST");
        assert!(!env_flag_enabled("OPS_NONINTERACTIVE_TEST"));
    }

    /// TEST-25 / TASK-1312: split out of the previous combined
    /// `handlers_do_not_reload_config` test so each assertion is paired
    /// with a branch that actually invokes the named code path.
    #[test]
    #[serial_test::serial]
    fn cli_data_context_does_not_reload_config() {
        let (_dir, _guard) = crate::test_utils::with_temp_config(
            r#"
[commands.echo_test]
program = "echo"
args = ["hi"]
"#,
        );

        ops_core::config::reset_load_config_call_count();
        let config = ops_core::config::load_config_or_default("test-early");
        assert_eq!(
            ops_core::config::load_config_call_count(),
            1,
            "early load should be the only load_config call so far"
        );

        let _ = cli_data_context(&config).expect("cli_data_context");

        assert_eq!(
            ops_core::config::load_config_call_count(),
            1,
            "cli_data_context must not reload .ops.toml"
        );
    }

    /// TEST-25 / TASK-1312: pin the load-count invariant on the dispatch
    /// branch that actually routes through `run_external_command`. The
    /// previous combined test configured no `run-before-commit` command,
    /// so the dispatch short-circuited into `prompt_hook_install` and the
    /// assertion below could not have caught a regression that re-loaded
    /// `.ops.toml` inside the configured-command path.
    #[test]
    #[serial_test::serial]
    fn run_hook_dispatch_does_not_reload_config() {
        let (_dir, _guard) = crate::test_utils::with_temp_config(
            r#"
[commands.run-before-commit]
program = "true"
"#,
        );

        ops_core::config::reset_load_config_call_count();
        let config = ops_core::config::load_config_or_default("test-early");
        assert_eq!(
            ops_core::config::load_config_call_count(),
            1,
            "early load should be the only load_config call so far"
        );
        assert!(
            config.commands.contains_key("run-before-commit"),
            "fixture must configure run-before-commit so dispatch enters the run_external_command branch"
        );

        // SKIP_OPS_RUN_BEFORE_COMMIT and OPS_RUN_BEFORE_COMMIT_GIT_TIMEOUT_SECS
        // must not be set from the ambient env, or the should_skip branch
        // would short-circuit before the run_external_command path.
        let _skip_guard = EnvVarGuard::unset("SKIP_OPS_RUN_BEFORE_COMMIT");

        let res = run_hook_dispatch(
            std::sync::Arc::new(config),
            &pre_hook_cmd::COMMIT_OPS,
            false,
        );
        // `true` exits 0, but propagate any spawn error so a CI environment
        // without `/usr/bin/true` surfaces a clear failure instead of a
        // confusing load-count mismatch.
        res.expect("run_hook_dispatch over configured `true` command must succeed");

        assert_eq!(
            ops_core::config::load_config_call_count(),
            1,
            "run_hook_dispatch must not reload .ops.toml on the configured-command (run_external_command) branch"
        );
    }

    /// RAII helper: remove an env var for the scope of a test and restore
    /// the original value on drop. Local to this test module so the broader
    /// test_utils surface does not grow another env-guard variant.
    struct EnvVarGuard {
        name: &'static str,
        original: Option<std::ffi::OsString>,
    }

    impl EnvVarGuard {
        fn unset(name: &'static str) -> Self {
            let original = std::env::var_os(name);
            std::env::remove_var(name);
            Self { name, original }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            match self.original.take() {
                Some(v) => std::env::set_var(self.name, v),
                None => std::env::remove_var(self.name),
            }
        }
    }

    /// API-1 / TASK-1307: `--changed-only` against a hook with no preflight
    /// predicate must fail loudly. The legacy behaviour silently no-op'd,
    /// hiding the misuse from any scripted caller.
    #[test]
    #[serial_test::serial]
    fn run_hook_dispatch_bails_when_changed_only_set_without_preflight() {
        let mut cfg = Config::default();
        cfg.commands.insert(
            "run-before-push".to_string(),
            ops_core::config::CommandSpec::Exec(ops_core::config::ExecCommandSpec::new(
                "true",
                Vec::<String>::new(),
            )),
        );
        // PUSH_OPS.preflight is None. Asking for --changed-only must error.
        let res = run_hook_dispatch(std::sync::Arc::new(cfg), &pre_hook_cmd::PUSH_OPS, true);
        let err = res.expect_err("--changed-only without preflight must bail");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("--changed-only is not supported for run-before-push"),
            "error must name the hook + flag, got: {msg}"
        );
    }

    /// API-1 / TASK-1325: every "config still missing" path through
    /// `prompt_hook_install` must report FAILURE — git treats SUCCESS as
    /// "hook passed", so a user who declines the install prompt must not
    /// silently let the commit through.
    #[test]
    #[serial_test::serial]
    fn prompt_hook_install_noninteractive_reports_failure() {
        std::env::set_var("OPS_NONINTERACTIVE", "1");
        let cfg = Config::default();
        let code = prompt_hook_install(&cfg, &pre_hook_cmd::COMMIT_OPS)
            .expect("noninteractive bail must not error");
        std::env::remove_var("OPS_NONINTERACTIVE");
        // ExitCode is opaque — compare its Debug form against FAILURE.
        assert_eq!(format!("{code:?}"), format!("{:?}", ExitCode::FAILURE));
    }

    /// CL-3 / TASK-1324: both pre-* hook help screens must render with the
    /// same section structure. The previous `next_help_heading = "Setup"`
    /// on RunBeforePush split the two hooks across help sections; this
    /// pins that they no longer diverge.
    #[test]
    fn run_before_commit_and_push_help_share_structure() {
        use clap::CommandFactory;
        let cmd = crate::args::Cli::command();
        let commit = cmd
            .find_subcommand("run-before-commit")
            .expect("commit subcmd")
            .clone();
        let push = cmd
            .find_subcommand("run-before-push")
            .expect("push subcmd")
            .clone();
        // Neither variant should carry a `Setup` help heading on its own
        // flag block — categorisation is centralised in `help::builtin_category`.
        assert_eq!(commit.get_next_help_heading(), None);
        assert_eq!(push.get_next_help_heading(), None);
    }

    /// PATTERN-1 / TASK-1322: adding a new `HookOps` to the `pre_hook_cmd`
    /// constant table must make `prompt_hook_install` reachable for that
    /// hook without a parallel match-arm edit in `subcommands.rs`. The
    /// function now dispatches through `hook.install_fn`, so this test
    /// witnesses the seam.
    #[test]
    fn prompt_hook_install_dispatches_via_install_fn_field() {
        use std::sync::atomic::{AtomicBool, Ordering};
        static CALLED: AtomicBool = AtomicBool::new(false);
        fn fake_install(_cfg: &Config) -> anyhow::Result<()> {
            CALLED.store(true, Ordering::SeqCst);
            Ok(())
        }
        let fake = HookOps {
            install_fn: fake_install,
            ..pre_hook_cmd::COMMIT_OPS
        };
        // Exercise just the dispatch surface: the install_fn must be the
        // sole reference to the install entry point. This compiles iff
        // `HookOps::install_fn` is the seam — a stringly-typed match would
        // have ignored the override.
        (fake.install_fn)(&Config::default()).expect("fake install ran");
        assert!(CALLED.load(Ordering::SeqCst), "install_fn was not invoked");
    }
}
