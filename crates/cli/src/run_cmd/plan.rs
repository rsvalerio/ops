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
pub(crate) fn merge_plan(
    runner: &ops_runner::command::CommandRunner,
    names: &[&str],
) -> anyhow::Result<(Vec<ops_core::config::CommandId>, bool, bool)> {
    let mut all_leaf_ids = Vec::new();
    let mut any_parallel = false;
    let mut fail_fast = true;
    for name in names {
        let leaf_ids = runner.expand_to_leaves(name).map_err(anyhow::Error::from)?;
        all_leaf_ids.extend(leaf_ids);
        let (has_parallel, fail_fast_disabled) = super::composite_tree_flags(runner, name);
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
    match runner.resolve(id) {
        Some(CommandSpec::Exec(e)) => e.display_cmd().into_owned(),
        _ => id.to_string(),
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
