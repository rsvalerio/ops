//! Plan assembly: leaf-id expansion, display-map construction, step logging.

use ops_core::config::CommandSpec;
use ops_runner::command::StepResult;

/// Merge leaf IDs from multiple command names into a single plan.
///
/// PATTERN-1 / TASK-0754: aggregation walks each name's composite tree so a
/// nested composite with `parallel = true` or `fail_fast = false` is
/// honoured. The earlier shape only inspected the top-level composite for
/// each `name`, silently dropping nested parallelism / fail-fast semantics
/// for `umbrella = { commands = ["inner"] }` where `inner.parallel = true`.
///
/// PATTERN-1 / TASK-1091: an empty `names` slice is rejected with an error.
/// The previous shape returned `(empty_plan, any_parallel = false,
/// fail_fast = true)`, and the executor then ran zero steps and reported
/// success. That silent "ran nothing, success" outcome masks upstream
/// filtering bugs (callers that ended up with an empty argv after CLI
/// parsing or hook filtering). The single production caller
/// [`run_external_command`] already rejects empty argv before reaching
/// here, so the error path is a defensive fail-loud guard rather than a
/// behavioural change for the happy path.
pub(crate) fn merge_plan(
    runner: &ops_runner::command::CommandRunner,
    names: &[&str],
) -> anyhow::Result<(Vec<ops_core::config::CommandId>, bool, bool)> {
    if names.is_empty() {
        anyhow::bail!(
            "merge_plan called with empty names slice — refusing to plan zero commands \
             (this would otherwise execute zero steps and report success, masking an \
             upstream filtering bug)"
        );
    }
    // PATTERN-1 / TASK-1283: a single traversal per name returns both the
    // leaf ids and the (any_parallel, fail_fast_disabled) flags, so the
    // executed leaf set and the plan flags are derived from the same walk
    // (no risk of independent walks drifting in cycle/ordering semantics).
    let mut all_leaf_ids = Vec::new();
    let mut any_parallel = false;
    let mut fail_fast = true;
    for name in names {
        let (leaf_ids, has_parallel, fail_fast_disabled) = runner
            .expand_to_leaves_with_flags(name)
            .map_err(anyhow::Error::from)?;
        all_leaf_ids.extend(leaf_ids);
        if has_parallel {
            any_parallel = true;
        }
        if fail_fast_disabled {
            fail_fast = false;
        }
    }
    Ok((all_leaf_ids, any_parallel, fail_fast))
}

pub(crate) fn display_cmd_for(runner: &ops_runner::command::CommandRunner, id: &str) -> String {
    // READ-7 / TASK-0903: match every CommandSpec variant explicitly so a
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
pub(crate) fn build_display_map(
    runner: &ops_runner::command::CommandRunner,
    leaf_ids: &[ops_core::config::CommandId],
) -> std::collections::HashMap<String, String> {
    leaf_ids
        .iter()
        .map(|id| (id.to_string(), display_cmd_for(runner, id)))
        .collect()
}

/// Log step results at debug level.
pub(crate) fn log_step_results(results: &[StepResult]) {
    for r in results {
        tracing::debug!(
            id = %r.id,
            success = r.success,
            duration_ms = r.duration.as_millis() as u64,
            stdout_len = r.stdout.len(),
            stderr_len = r.stderr.len(),
            message = ?r.message,
            "step result",
        );
    }
}
