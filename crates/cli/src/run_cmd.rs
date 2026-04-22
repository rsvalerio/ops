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

pub(crate) fn run_external_command(
    args: &[OsString],
    dry_run: bool,
    verbose: bool,
    tap: Option<PathBuf>,
    raw: bool,
) -> anyhow::Result<ExitCode> {
    let names: Vec<&str> = args.iter().filter_map(|s| s.to_str()).collect();
    if names.is_empty() {
        anyhow::bail!("missing command name");
    }
    if names.len() == 1 {
        return run_command(names[0], dry_run, verbose, tap, raw);
    }
    run_commands(&names, dry_run, verbose, tap, raw)
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
fn run_with_runtime<F, T>(f: F) -> anyhow::Result<T>
where
    F: std::future::Future<Output = anyhow::Result<T>>,
{
    tokio::runtime::Runtime::new()?.block_on(f)
}

fn run_commands(
    names: &[&str],
    dry_run: bool,
    verbose: bool,
    tap: Option<PathBuf>,
    raw: bool,
) -> anyhow::Result<ExitCode> {
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
fn run_command(
    name: &str,
    dry_run: bool,
    verbose: bool,
    tap: Option<PathBuf>,
    raw: bool,
) -> anyhow::Result<ExitCode> {
    let mut runner = build_runner(verbose)?;

    if dry_run {
        return run_command_dry_run(&runner, name);
    }

    let success = if raw {
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
    let results: Vec<StepResult> = run_with_runtime(async { runner.run_raw(name).await })?;
    log_step_results(&results);
    Ok(results.iter().all(|r| r.success))
}

fn run_command_cli(
    runner: &mut ops_runner::command::CommandRunner,
    name: &str,
    tap: Option<PathBuf>,
) -> anyhow::Result<bool> {
    let leaf_ids = runner
        .expand_to_leaves(name)
        .ok_or_else(|| anyhow::anyhow!("unknown command: {}", name))?;

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
