//! Plan assembly: leaf-id expansion, display-map construction, step logging.

use ops_core::config::CommandSpec;
use ops_runner::command::StepResult;

/// One named command's own execution plan (TASK-2262).
///
/// `ops run a b` expands each name independently, so `parallel`, `fail_fast`
/// (and, for builtins, `exclusive`) come from each name's own composite tree
/// and are never merged across names. The earlier `merge_plan` shape folded
/// every name into one flat plan whose `any_parallel` was true whenever any
/// name's was, so a single parallel name (a rust-stack `verify`) promoted the
/// steps of every other named command — including ones that declared
/// `parallel = false` — into concurrent execution.
#[derive(Debug, Clone)]
pub struct NamePlan {
    /// The name exactly as the user invoked it, for diagnostics.
    pub name: String,
    /// Exec-only leaf ids for this name, in declaration order.
    pub leaf_ids: Vec<ops_core::config::CommandId>,
    /// Whether this name's own tree schedules its steps in parallel.
    pub any_parallel: bool,
    /// Whether this name's own tree stops on the first failing step.
    pub fail_fast: bool,
}

/// Expand each named command into its own plan.
///
/// Aggregation walks each name's composite tree so a nested composite
/// with `parallel = true` or `fail_fast = false` is honoured. The earlier
/// shape only inspected the top-level composite for each `name`, silently
/// dropping nested parallelism / fail-fast semantics for
/// `umbrella = { commands = ["inner"] }` where `inner.parallel = true`.
///
/// An empty `names` slice is rejected with an error.
/// The previous shape returned `(empty_plan, any_parallel = false,
/// fail_fast = true)`, and the executor then ran zero steps and reported
/// success. That silent "ran nothing, success" outcome masks upstream
/// filtering bugs (callers that ended up with an empty argv after CLI
/// parsing or hook filtering). The single production caller
/// [`super::run_external_command`] already rejects empty argv before reaching
/// here, so the error path is a defensive fail-loud guard rather than a
/// behavioural change for the happy path.
pub fn plans_for_names(
    runner: &ops_runner::command::CommandRunner,
    names: &[&str],
) -> anyhow::Result<Vec<NamePlan>> {
    if names.is_empty() {
        anyhow::bail!(
            "plans_for_names called with empty names slice — refusing to plan zero commands \
             (this would otherwise execute zero steps and report success, masking an \
             upstream filtering bug)"
        );
    }
    // A single traversal per name returns both the
    // leaf ids and the (any_parallel, fail_fast_disabled) flags, so the
    // executed leaf set and the plan flags are derived from the same walk
    // (no risk of independent walks drifting in cycle/ordering semantics).
    // Every name is expanded up front, before the executor starts, so an
    // unknown or cyclic name anywhere in the invocation fails the whole run
    // instead of surfacing after earlier names have already executed.
    let mut plans = Vec::with_capacity(names.len());
    for name in names {
        let (leaf_ids, any_parallel, fail_fast_disabled) = runner
            .expand_to_leaves_with_flags(name)
            .map_err(anyhow::Error::from)?;
        plans.push(NamePlan {
            name: (*name).to_string(),
            leaf_ids,
            any_parallel,
            fail_fast: !fail_fast_disabled,
        });
    }
    Ok(plans)
}

pub fn display_cmd_for(runner: &ops_runner::command::CommandRunner, id: &str) -> String {
    // Match every CommandSpec variant explicitly so a
    // future variant fails to compile here rather than silently falling
    // back to the bare id in plan rows. Composites surface a comma-joined
    // child list (mirrors `display_cmd_fallback`) which is what plan
    // display rows want — the bare id told the user nothing.
    match runner.resolve(id) {
        Some(CommandSpec::Exec(e)) => e.display_cmd().into_owned(),
        Some(CommandSpec::Composite(c)) => c.commands.join(", "),
        None => id.to_string(),
    }
}

/// Build a display map from command IDs to their display strings.
pub fn build_display_map(
    runner: &ops_runner::command::CommandRunner,
    leaf_ids: &[ops_core::config::CommandId],
) -> std::collections::HashMap<String, String> {
    leaf_ids
        .iter()
        .map(|id| (id.to_string(), display_cmd_for(runner, id)))
        .collect()
}

/// Log step results at debug level.
pub fn log_step_results(results: &[StepResult]) {
    for r in results {
        tracing::debug!(
            id = %r.id,
            success = r.success,
            duration_ms = u64::try_from(r.duration.as_millis()).unwrap_or(u64::MAX),
            stdout_len = r.stdout.len(),
            stderr_len = r.stderr.len(),
            message = ?r.message,
            "step result",
        );
    }
}
