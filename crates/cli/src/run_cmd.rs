//! Command resolution, execution, dry-run preview, and display helpers.
//!
//! Split for cohesion:
//! - [`dry_run`] — resolve and print commands without executing
//! - [`plan`]    — leaf-id expansion, display-map, step logging

mod dry_run;
mod plan;
#[cfg(test)]
mod tests;

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;

use ops_runner::command::StepResult;
use ops_runner::display::{DisplayOptions, ProgressDisplay};
use ops_runner::terminal::EchoGuard;

use crate::registry::{as_ext_refs, builtin_extensions, register_extension_commands};

use dry_run::run_command_dry_run;
use plan::{build_display_map, log_step_results, merge_plan};

/// Options for a top-level `run` invocation, threaded through the
/// `run_command` / `run_commands` helpers. FN-3 / TASK-0272: collapses five
/// positional args (including three `bool`s) into a named struct so swap
/// bugs like `run_command(name, true, false, …)` — was that dry_run or
/// verbose? — become impossible at call sites.
#[derive(Debug, Clone, Default)]
pub(crate) struct RunOptions {
    pub dry_run: bool,
    pub verbose: bool,
    pub tap: Option<PathBuf>,
    pub raw: bool,
}

pub(crate) fn run_external_command(
    args: &[OsString],
    opts: RunOptions,
) -> anyhow::Result<ExitCode> {
    // API-1: report non-UTF-8 argv entries explicitly. Previously a bad
    // OsString silently vanished via `filter_map(OsStr::to_str)` and the
    // user saw a generic "missing command name" when that left zero args.
    let mut names: Vec<&str> = Vec::with_capacity(args.len());
    for a in args {
        match a.to_str() {
            Some(s) => names.push(s),
            None => anyhow::bail!(
                "command name contains non-UTF-8 bytes: {a:?} — ops command names must be UTF-8"
            ),
        }
    }
    if names.is_empty() {
        anyhow::bail!("missing command name");
    }
    if names.len() == 1 {
        return run_command(names[0], opts);
    }
    run_commands(&names, opts)
}

fn build_runner(verbose: bool) -> anyhow::Result<ops_runner::command::CommandRunner> {
    let (mut config, cwd) = crate::load_config_and_cwd()?;
    if verbose {
        config.output.stderr_tail_lines = usize::MAX;
    }
    let mut runner = ops_runner::command::CommandRunner::new(config, cwd);
    setup_extensions(&mut runner)?;
    Ok(runner)
}

/// Create a tokio Runtime, run the async closure on it, and return the result.
///
/// Wraps `Runtime::new()` with `.context(...)` so resource-limit failures
/// (EMFILE, ENOMEM, epoll init errors) surface with a message explaining
/// *why* the runtime is being started, rather than a bare
/// `Too many open files (os error 24)` that the user cannot correlate back
/// to `ops run …`. See ERR-1 / TASK-0160.
fn run_with_runtime<F, T>(f: F) -> anyhow::Result<T>
where
    F: std::future::Future<Output = anyhow::Result<T>>,
{
    use anyhow::Context as _;
    tokio::runtime::Runtime::new()
        .context("failed to start tokio runtime for command execution")?
        .block_on(f)
}

fn run_commands(names: &[&str], opts: RunOptions) -> anyhow::Result<ExitCode> {
    let RunOptions {
        dry_run,
        verbose,
        tap,
        raw,
    } = opts;
    let runner = build_runner(verbose)?;

    if dry_run {
        for name in names {
            run_command_dry_run(&runner, name)?;
        }
        return Ok(ExitCode::SUCCESS);
    }

    let (all_leaf_ids, any_parallel, fail_fast) = merge_plan(&runner, names)?;

    if raw {
        // Raw mode: child owns the terminal, no display is attached, and
        // composite `parallel = true` is ignored (sequential only).
        if any_parallel {
            tracing::warn!(
                "--raw forces sequential execution; composite `parallel = true` is ignored"
            );
        }
        // READ-10: there is no stream to tap in raw mode (child stdio is
        // inherited directly). Warn so users combining the flags see the
        // contradiction rather than getting a silent no-op or an empty
        // file somewhere.
        if tap.is_some() {
            tracing::warn!(
                "--tap is ignored under --raw because raw mode inherits child stdio; no tap file will be written"
            );
        }
        let results: Vec<StepResult> =
            run_with_runtime(async { Ok(runner.run_plan_raw(&all_leaf_ids, fail_fast).await) })?;
        log_step_results(&results);
        let success = results.iter().all(|r| r.success);
        return Ok(if success {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        });
    }
    let display_map = build_display_map(&runner, &all_leaf_ids);
    let mut display = ProgressDisplay::new(DisplayOptions {
        output: runner.output_config(),
        display_map,
        custom_themes: &runner.config().themes,
        tap,
    })?;

    let _echo_guard = EchoGuard::disable_echo();
    let results: Vec<StepResult> = run_with_runtime(async {
        Ok(if any_parallel {
            runner
                .run_plan_parallel(&all_leaf_ids, fail_fast, &mut |event| {
                    display.handle_event(event)
                })
                .await
        } else {
            runner
                .run_plan(&all_leaf_ids, fail_fast, &mut |event| {
                    display.handle_event(event)
                })
                .await
        })
    })?;
    drop(_echo_guard);
    log_step_results(&results);

    let success = results.iter().all(|r| r.success);
    Ok(if success {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

fn setup_extensions(runner: &mut ops_runner::command::CommandRunner) -> anyhow::Result<()> {
    let exts = builtin_extensions(runner.config(), runner.working_directory())?;
    let ext_refs = as_ext_refs(&exts);
    let mut cmd_registry = ops_extension::CommandRegistry::new();
    register_extension_commands(&ext_refs, &mut cmd_registry);
    runner.register_commands(cmd_registry);
    let mut data_registry = ops_extension::DataRegistry::new();
    crate::registry::register_extension_data_providers(&ext_refs, &mut data_registry);
    runner.register_data_providers(data_registry);
    Ok(())
}

#[tracing::instrument(skip_all, fields(command = %name))]
fn run_command(name: &str, opts: RunOptions) -> anyhow::Result<ExitCode> {
    let RunOptions {
        dry_run,
        verbose,
        tap,
        raw,
    } = opts;
    let mut runner = build_runner(verbose)?;

    if dry_run {
        return run_command_dry_run(&runner, name);
    }

    let success = if raw {
        if tap.is_some() {
            tracing::warn!(
                "--tap is ignored under --raw because raw mode inherits child stdio; no tap file will be written"
            );
        }
        run_command_raw(&runner, name)?
    } else {
        run_command_cli(&mut runner, name, tap)?
    };

    if success {
        Ok(ExitCode::SUCCESS)
    } else {
        Ok(ExitCode::FAILURE)
    }
}

fn run_command_raw(
    runner: &ops_runner::command::CommandRunner,
    name: &str,
) -> anyhow::Result<bool> {
    // READ-10: parity with the multi-command path in `run_commands`. When
    // `--raw` is combined with a composite that sets `parallel = true`, the
    // raw runner forces sequential execution; warn so the user does not
    // silently get serialized timing for a parallel-annotated composite.
    warn_raw_drops_parallel(runner, name);
    let results: Vec<StepResult> = run_with_runtime(async { runner.run_raw(name).await })?;
    log_step_results(&results);
    Ok(results.iter().all(|r| r.success))
}

/// If `name` resolves to a composite with `parallel = true`, emit the same
/// warning the multi-command raw path already emits. No-op otherwise so
/// leaf commands and sequential composites stay quiet.
fn warn_raw_drops_parallel(runner: &ops_runner::command::CommandRunner, name: &str) {
    if let Some(ops_core::config::CommandSpec::Composite(c)) = runner.resolve(name) {
        if c.parallel {
            tracing::warn!(
                command = %name,
                "--raw forces sequential execution; composite `parallel = true` is ignored"
            );
        }
    }
}

fn run_command_cli(
    runner: &mut ops_runner::command::CommandRunner,
    name: &str,
    tap: Option<PathBuf>,
) -> anyhow::Result<bool> {
    // ERR-10: surface the specific expansion failure (unknown/cycle/
    // depth-exceeded) via the typed `ExpandError`, instead of rewriting
    // every case to "unknown command".
    let leaf_ids = runner.expand_to_leaves(name).map_err(anyhow::Error::from)?;

    let display_map = build_display_map(runner, &leaf_ids);

    let mut display = ProgressDisplay::new(DisplayOptions {
        output: runner.output_config(),
        display_map,
        custom_themes: &runner.config().themes,
        tap,
    })?;

    let _echo_guard = EchoGuard::disable_echo();
    let results: Vec<StepResult> = run_with_runtime(async {
        runner
            .run(name, &mut |event| display.handle_event(event))
            .await
    })?;
    drop(_echo_guard);
    log_step_results(&results);

    let success = results.iter().all(|r| r.success);
    Ok(success)
}
