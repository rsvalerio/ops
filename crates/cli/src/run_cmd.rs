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
    /// SEC-14 / TASK-0886: cwd-escape policy applied to the runner this
    /// invocation builds. Hook-triggered entry points
    /// (`run-before-commit`, `run-before-push`) set
    /// `CwdEscapePolicy::Deny` so a coworker-landed `.ops.toml` cannot
    /// escape the workspace on the next commit. Default
    /// (`CwdEscapePolicy::WarnAndAllow`) preserves the interactive trust
    /// model for `ops <cmd>`.
    pub cwd_escape_policy: ops_runner::command::CwdEscapePolicy,
}

pub(crate) fn run_external_command(
    config: std::sync::Arc<ops_core::config::Config>,
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
        return run_command(config, names[0], opts);
    }
    run_commands(config, &names, opts)
}

/// OWN-2 / TASK-0841: take an `Arc<Config>` shared with `main`/`dispatch`
/// rather than deep-cloning the Config per invocation. The runner shares
/// the same allocation as the early-loaded config — every nested
/// `IndexMap`, `String`, theme block is allocated exactly once per CLI run.
fn build_runner(
    config: std::sync::Arc<ops_core::config::Config>,
    _verbose: bool,
    cwd_escape_policy: ops_runner::command::CwdEscapePolicy,
) -> anyhow::Result<ops_runner::command::CommandRunner> {
    let cwd = crate::cwd()?;
    let mut runner = ops_runner::command::CommandRunner::from_arc_config(config, cwd);
    runner.set_cwd_escape_policy(cwd_escape_policy);
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
    // Conservative default: paths that have not analysed the plan keep the
    // multi-thread runtime so a composite that fans out internally still has
    // worker threads available. Callers that have proved the plan is purely
    // sequential pass [`RuntimeKind::Sequential`] explicitly.
    run_with_runtime_kind(RuntimeKind::MultiThread, f)
}

/// ASYNC-7 / TASK-0875: which runtime flavour the run path needs.
///
/// Sequential CLI invocations don't fan out to a worker pool, so paying
/// to spin up `worker_thread × CPU` for a single short command is pure
/// startup overhead. The parallel orchestrator does need real worker
/// parallelism — `spawn_parallel_tasks` schedules each child on the
/// runtime — so callers that take the parallel path must use
/// [`RuntimeKind::MultiThread`].
#[derive(Clone, Copy)]
enum RuntimeKind {
    Sequential,
    MultiThread,
}

fn run_with_runtime_kind<F, T>(kind: RuntimeKind, f: F) -> anyhow::Result<T>
where
    F: std::future::Future<Output = anyhow::Result<T>>,
{
    use anyhow::Context as _;
    let rt = match kind {
        RuntimeKind::Sequential => tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build(),
        RuntimeKind::MultiThread => tokio::runtime::Runtime::new(),
    };
    rt.context("failed to start tokio runtime for command execution")?
        .block_on(f)
}

fn run_commands(
    config: std::sync::Arc<ops_core::config::Config>,
    names: &[&str],
    opts: RunOptions,
) -> anyhow::Result<ExitCode> {
    let RunOptions {
        dry_run,
        verbose,
        tap,
        raw,
        cwd_escape_policy,
    } = opts;
    let runner = build_runner(config, verbose, cwd_escape_policy)?;

    if dry_run {
        // ERR-1 / TASK-1234: extend the execute path's `emit_raw_warnings`
        // contract to dry-run so users invoking `ops <cmd> --dry-run --raw
        // --tap=path` see that --raw/--tap have no effect, instead of a
        // silent override.
        emit_dry_run_warnings(raw, tap.is_some());
        for name in names {
            run_command_dry_run(&runner, name)?;
        }
        return Ok(ExitCode::SUCCESS);
    }

    let (all_leaf_ids, any_parallel, fail_fast) = merge_plan(&runner, names)?;
    let plan = PlanShape {
        leaf_ids: &all_leaf_ids,
        any_parallel,
        fail_fast,
    };

    let results = if raw {
        run_commands_raw(&runner, plan, tap.as_ref())?
    } else {
        run_commands_with_display(&runner, plan, tap, verbose)?
    };
    Ok(summarize(&results))
}

/// FN-3 / TASK-0866: shape of a planned execution. Grouping the three
/// related fields removes the adjacent `bool, bool` swap footgun from
/// every plan-running entry point and keeps `run_commands_raw` and
/// `run_commands_with_display` in lock-step on what a "plan" is.
#[derive(Clone, Copy)]
struct PlanShape<'a> {
    leaf_ids: &'a [ops_core::config::CommandId],
    any_parallel: bool,
    fail_fast: bool,
}

fn run_commands_raw(
    runner: &ops_runner::command::CommandRunner,
    plan: PlanShape<'_>,
    tap: Option<&PathBuf>,
) -> anyhow::Result<Vec<StepResult>> {
    emit_raw_warnings(plan.any_parallel, tap.is_some());
    let results: Vec<StepResult> =
        run_with_runtime(async { Ok(runner.run_plan_raw(plan.leaf_ids, plan.fail_fast).await) })?;
    log_step_results(&results);
    Ok(results)
}

/// ERR-1 / TASK-1234: messages emitted when `--dry-run` is combined with
/// otherwise-incompatible flags. The dry-run path materialises a preview
/// without consulting `--raw` or `--tap`; without this contract, a user
/// invoking `ops <cmd> --dry-run --raw --tap=path` saw no indication that
/// those flags had no effect, mirroring the silent-override bug the
/// execute-path `emit_raw_warnings` was introduced to fix.
///
/// Returns the static messages a caller would log so a unit test can
/// assert the contract without intercepting a tracing subscriber.
fn dry_run_overrides_messages(raw: bool, has_tap: bool) -> Vec<&'static str> {
    let mut msgs = Vec::new();
    if raw {
        msgs.push(
            "--raw is ignored under --dry-run; the dry-run preview never executes children, \
             so raw-mode stdio inheritance is not exercised",
        );
    }
    if has_tap {
        msgs.push(
            "--tap is ignored under --dry-run; the dry-run preview never executes children, \
             so no tap file will be written",
        );
    }
    msgs
}

fn emit_dry_run_warnings(raw: bool, has_tap: bool) {
    for m in dry_run_overrides_messages(raw, has_tap) {
        tracing::warn!("{m}");
    }
}

/// Warnings emitted when `--raw` is combined with otherwise-incompatible
/// flags. Extracted from `run_commands_raw` so the warning logic itself can
/// be unit-tested without spinning up a full runner.
fn emit_raw_warnings(any_parallel: bool, has_tap: bool) {
    // Raw mode forces sequential execution; `parallel = true` composites
    // are ignored.
    if any_parallel {
        tracing::warn!("--raw forces sequential execution; composite `parallel = true` is ignored");
    }
    // READ-10: there is no stream to tap in raw mode (child stdio is
    // inherited directly). Warn so users combining the flags see the
    // contradiction rather than getting a silent no-op or an empty file
    // somewhere.
    if has_tap {
        tracing::warn!(
            "--tap is ignored under --raw because raw mode inherits child stdio; no tap file will be written"
        );
    }
}

fn run_commands_with_display(
    runner: &ops_runner::command::CommandRunner,
    plan: PlanShape<'_>,
    tap: Option<PathBuf>,
    verbose: bool,
) -> anyhow::Result<Vec<StepResult>> {
    let display_map = build_display_map(runner, plan.leaf_ids);
    let mut display = ProgressDisplay::new(DisplayOptions::new(
        runner.output_config(),
        display_map,
        &runner.config().themes,
        tap,
        verbose,
    ))?;

    let _echo_guard = EchoGuard::disable_echo();
    // ASYNC-7 / TASK-1138: parallel orchestration only pays off with >=2 leaves;
    // a 1-leaf plan with `parallel = true` shortcuts to `run_plan` in
    // `run_plan_parallel` (parallel.rs), so picking MultiThread here would
    // pay worker-thread spin-up for nothing. Mirror that threshold.
    let kind = if plan.any_parallel && plan.leaf_ids.len() > 1 {
        RuntimeKind::MultiThread
    } else {
        RuntimeKind::Sequential
    };
    let results: Vec<StepResult> = run_with_runtime_kind(kind, async {
        Ok(if plan.any_parallel {
            runner
                .run_plan_parallel(plan.leaf_ids, plan.fail_fast, &mut |event| {
                    display.handle_event(event)
                })
                .await
        } else {
            runner
                .run_plan(plan.leaf_ids, plan.fail_fast, &mut |event| {
                    display.handle_event(event)
                })
                .await
        })
    })?;
    drop(_echo_guard);
    log_step_results(&results);
    Ok(results)
}

fn summarize(results: &[StepResult]) -> ExitCode {
    if results.iter().all(|r| r.success) {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
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
    config: std::sync::Arc<ops_core::config::Config>,
    name: &str,
    opts: RunOptions,
) -> anyhow::Result<ExitCode> {
    let RunOptions {
        dry_run,
        verbose,
        tap,
        raw,
        cwd_escape_policy,
    } = opts;
    let mut runner = build_runner(config, verbose, cwd_escape_policy)?;

    if dry_run {
        // ERR-1 / TASK-1234: mirror the multi-command path so the single-
        // command dry-run branch surfaces the same --raw/--tap override
        // diagnostic the execute path already emits.
        emit_dry_run_warnings(raw, tap.is_some());
        return run_command_dry_run(&runner, name);
    }

    let success = if raw {
        run_command_raw(&runner, name, tap.is_some())?
    } else {
        run_command_cli(&mut runner, name, tap, verbose)?
    };

    Ok(if success {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

fn run_command_raw(
    runner: &ops_runner::command::CommandRunner,
    name: &str,
    has_tap: bool,
) -> anyhow::Result<bool> {
    // CL-5 / TASK-0755: route both raw-mode warnings through
    // `emit_raw_warnings` so the message strings live in exactly one place.
    // Earlier the tap warning was inlined here while parallel-detection went
    // through a separate helper, leaving the two raw paths free to drift.
    emit_raw_warnings(composite_tree_has_parallel(runner, name), has_tap);
    let results: Vec<StepResult> = run_with_runtime(async { runner.run_raw(name).await })?;
    log_step_results(&results);
    Ok(results.iter().all(|r| r.success))
}

pub(super) fn composite_tree_has_parallel(
    runner: &ops_runner::command::CommandRunner,
    name: &str,
) -> bool {
    composite_tree_flags(runner, name).0
}

/// Walk the composite tree rooted at `name` and report whether any node has
/// `parallel = true` or `fail_fast = false`. PATTERN-1 / TASK-0754:
/// `merge_plan` previously inspected only the top-level composite for these
/// flags, dropping nested parallelism / fail-fast aggregation silently. The
/// raw single-command path already walked the tree for its `parallel`
/// warning; callers that need the same semantics for `fail_fast` use the
/// second tuple element.
pub(super) fn composite_tree_flags(
    runner: &ops_runner::command::CommandRunner,
    name: &str,
) -> (bool, bool) {
    let mut visited: std::collections::HashSet<&str> = std::collections::HashSet::new();
    let mut stack: Vec<&str> = vec![name];
    let mut has_parallel = false;
    let mut fail_fast_disabled = false;
    while let Some(current) = stack.pop() {
        if !visited.insert(current) {
            continue;
        }
        if let Some(ops_core::config::CommandSpec::Composite(c)) = runner.resolve(current) {
            if c.parallel {
                has_parallel = true;
            }
            if !c.fail_fast {
                fail_fast_disabled = true;
            }
            if has_parallel && fail_fast_disabled {
                break;
            }
            for child in &c.commands {
                if !visited.contains(child.as_str()) {
                    stack.push(child.as_str());
                }
            }
        }
    }
    (has_parallel, fail_fast_disabled)
}

fn run_command_cli(
    runner: &mut ops_runner::command::CommandRunner,
    name: &str,
    tap: Option<PathBuf>,
    verbose: bool,
) -> anyhow::Result<bool> {
    // ERR-10: surface the specific expansion failure (unknown/cycle/
    // depth-exceeded) via the typed `ExpandError`, instead of rewriting
    // every case to "unknown command".
    let leaf_ids = runner.expand_to_leaves(name).map_err(anyhow::Error::from)?;

    let display_map = build_display_map(runner, &leaf_ids);

    let mut display = ProgressDisplay::new(DisplayOptions::new(
        runner.output_config(),
        display_map,
        &runner.config().themes,
        tap,
        verbose,
    ))?;

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
