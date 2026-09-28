//! `ops explain <cmd>...`: the resolved execution plan, never executed
//! (TASK-2280).
//!
//! The plan comes from the same [`super::plan::plans_for_names`] expansion the run path
//! uses, so what is printed is what `ops <cmd>...` would schedule: each named
//! command's plan tree, every composite's `parallel` / `fail_fast`, the
//! stages a parallel group splits into at its `exclusive` steps, and each
//! step's program, args, env, cwd and origin. Nothing here spawns a process:
//! the module only resolves specs and renders them. The one exception is
//! opt-in: `--json --tool-versions` hands [`write_json`] a version prober
//! that runs each listed tool's `--version` (TASK-2335).
//!
//! The subcommand is `explain` rather than `plan` because the terraform
//! stack ships a `plan` command, which a builtin `plan` would shadow.

use std::collections::HashSet;
use std::io::Write;

use ops_core::config::{CommandSpec, Config, ExecCommandSpec};
use ops_core::expand::Variables;
use ops_runner::command::{stage_lengths, CommandPlan, CommandRunner, CommandSource};
use serde_json::{json, Map, Value};

use super::dry_run::{audit_safe, env_display_value};
use super::plan::NamePlan;
use super::tools::{exec_tools, Tool};

/// Version of the `--json` document; bump on any breaking shape change.
pub const SCHEMA_VERSION: u32 = 1;

/// Answers the installed version of one tool, for `installedVersion`.
pub type VersionProbe<'a> = &'a dyn Fn(&Tool) -> Option<String>;

/// Render the plans as the versioned JSON document.
///
/// # Errors
///
/// A step's program, args, env or cwd references a variable that does not
/// expand, a matrix strategy is malformed, or writing `w` fails.
pub fn write_json(
    runner: &CommandRunner,
    plans: &[NamePlan],
    probe: Option<VersionProbe<'_>>,
    w: &mut dyn Write,
) -> anyhow::Result<()> {
    let doc = plan_document(runner, plans, probe)?;
    serde_json::to_writer_pretty(&mut *w, &doc)?;
    writeln!(w)?;
    Ok(())
}

/// Build the `--json` document (see [`write_json`]).
///
/// `steps` and `composites` are keyed by canonical name and listed once each
/// in first-use order, so a step shared by two named commands is described
/// once and referenced by id from each plan.
///
/// `tools` (TASK-2326) lists every external binary the plan needs on `PATH`,
/// once each in first-use order, with the steps that need it (see
/// [`super::tools`] for how they are derived). With a `probe`
/// (`--tool-versions`, TASK-2335) each tool also carries `installedVersion`:
/// what the probe answered, or `null` when the tool is missing or its
/// version could not be read. Without one the key is absent and nothing is
/// spawned. ops declares no minimum version for any tool, so none is
/// reported.
///
/// # Errors
///
/// As [`write_json`], minus the write.
pub fn plan_document(
    runner: &CommandRunner,
    plans: &[NamePlan],
    probe: Option<VersionProbe<'_>>,
) -> anyhow::Result<Value> {
    let config = runner.config();
    let vars = runner.variables();
    let mut composites = Vec::new();
    let mut seen_composites = HashSet::new();
    let mut steps = Vec::new();
    let mut seen_steps = HashSet::new();
    let mut commands = Vec::with_capacity(plans.len());
    let mut tools: Vec<(Tool, Vec<String>)> = Vec::new();

    for plan in plans {
        collect_composites(
            runner,
            config,
            &plan.name,
            &mut seen_composites,
            &mut composites,
        );
        for id in plan.plan.leaf_ids() {
            if seen_steps.insert(id.to_string()) {
                let (step, step_tools) = step_json(runner, config, vars, &id)?;
                steps.push(step);
                for tool in step_tools {
                    record_tool(&mut tools, tool, &id);
                }
            }
        }
        commands.push(json!({
            "name": plan.name,
            "plan": node_json(runner, &plan.plan),
        }));
    }

    Ok(json!({
        "schemaVersion": SCHEMA_VERSION,
        "kind": "command-plan",
        "commands": commands,
        "composites": composites,
        "steps": steps,
        "tools": tools
            .into_iter()
            .map(|(tool, required_by)| {
                let mut entry = json!({
                    "name": tool.name,
                    "optional": tool.optional,
                    "install": tool.install,
                    "requiredBy": required_by,
                });
                if let (Some(probe), Some(fields)) = (probe, entry.as_object_mut()) {
                    fields.insert("installedVersion".into(), json!(probe(&tool)));
                }
                entry
            })
            .collect::<Vec<_>>(),
    }))
}

/// Add `tool` as needed by step `id`. A tool is optional for the plan only
/// when every step that needs it can run without it.
fn record_tool(tools: &mut Vec<(Tool, Vec<String>)>, tool: Tool, id: &str) {
    match tools.iter_mut().find(|(t, _)| t.name == tool.name) {
        Some((known, required_by)) => {
            known.optional &= tool.optional;
            if known.install.is_none() {
                known.install = tool.install;
            }
            if !required_by.iter().any(|r| r == id) {
                required_by.push(id.to_string());
            }
        }
        None => tools.push((tool, vec![id.to_string()])),
    }
}

/// One plan-tree node. A stage lists its steps and the concurrent stages the
/// executor splits them into; a sequence lists its children in run order.
fn node_json(runner: &CommandRunner, plan: &CommandPlan) -> Value {
    match plan {
        CommandPlan::Stage {
            leaf_ids,
            parallel,
            fail_fast,
        } => json!({
            "type": "stage",
            "parallel": parallel,
            "failFast": fail_fast,
            "steps": leaf_ids.iter().map(ToString::to_string).collect::<Vec<_>>(),
            "stages": stages(runner, leaf_ids, *parallel)
                .into_iter()
                .map(|batch| json!({
                    "concurrent": batch.len() > 1,
                    "steps": batch,
                }))
                .collect::<Vec<_>>(),
        }),
        CommandPlan::Sequence {
            children,
            fail_fast,
        } => json!({
            "type": "sequence",
            "failFast": fail_fast,
            "children": children.iter().map(|c| node_json(runner, c)).collect::<Vec<_>>(),
        }),
    }
}

/// Split a stage's steps the way the executor does: a sequential stage runs
/// one step at a time; a parallel stage splits at every `exclusive` step
/// (see [`stage_lengths`]), and each run of consecutive non-exclusive steps
/// shares one concurrent batch.
fn stages(
    runner: &CommandRunner,
    leaf_ids: &[ops_core::config::CommandId],
    parallel: bool,
) -> Vec<Vec<String>> {
    let ids: Vec<String> = leaf_ids.iter().map(ToString::to_string).collect();
    if !parallel {
        return ids.into_iter().map(|id| vec![id]).collect();
    }
    let lengths = stage_lengths(ids.iter().map(|id| is_exclusive(runner, id)));
    let mut rest = ids.into_iter();
    lengths
        .into_iter()
        .map(|len| rest.by_ref().take(len).collect())
        .collect()
}

fn is_exclusive(runner: &CommandRunner, id: &str) -> bool {
    matches!(runner.resolve(id), Some(CommandSpec::Exec(e)) if e.exclusive)
}

/// Walk `id`'s composite tree, recording each composite once with its own
/// scheduling flags, entries and origin. Unknown names and cycles cannot
/// occur here: [`super::plan::plans_for_names`] already expanded the same
/// tree and would have failed first.
fn collect_composites(
    runner: &CommandRunner,
    config: &Config,
    id: &str,
    seen: &mut HashSet<String>,
    out: &mut Vec<Value>,
) {
    let Some(CommandSpec::Composite(c)) = runner.resolve(id) else {
        return;
    };
    let name = runner.locate(id).map_or(id, |(canonical, _)| canonical);
    if !seen.insert(name.to_string()) {
        return;
    }
    out.push(json!({
        "name": name,
        "parallel": c.parallel,
        "failFast": c.fail_fast,
        "commands": c.commands,
        "origin": origin_json(runner, config, name),
    }));
    for child in &c.commands {
        collect_composites(runner, config, child, seen, out);
    }
}

fn step_json(
    runner: &CommandRunner,
    config: &Config,
    vars: &Variables,
    id: &str,
) -> anyhow::Result<(Value, Vec<Tool>)> {
    let mut step = Map::new();
    let mut tools = Vec::new();
    step.insert("id".into(), json!(id));
    step.insert("origin".into(), origin_json(runner, config, id));
    match runner.resolve(id) {
        Some(CommandSpec::Exec(e)) => {
            step.insert("exclusive".into(), json!(e.exclusive));
            merge_tools(&mut tools, exec_fields(&mut step, e, vars)?);
            if let Some(strategy) = &e.strategy {
                let cells = e
                    .matrix_cells(id)?
                    .into_iter()
                    .map(|cell| {
                        let mut obj = Map::new();
                        obj.insert("id".into(), json!(cell.id));
                        merge_tools(&mut tools, exec_fields(&mut obj, &cell.spec, vars)?);
                        Ok(Value::Object(obj))
                    })
                    .collect::<anyhow::Result<Vec<_>>>()?;
                // A matrix step needs what any of its cells needs.
                step.insert(
                    "tools".into(),
                    json!(tools.iter().map(|t| &t.name).collect::<Vec<_>>()),
                );
                step.insert(
                    "matrix".into(),
                    json!({
                        "maxParallel": strategy.max_parallel,
                        "failFast": strategy.fail_fast,
                        "cells": cells,
                    }),
                );
            }
        }
        // Expansion only yields exec leaves; anything else is an internal
        // inconsistency worth surfacing rather than hiding.
        other => anyhow::bail!(
            "internal error: plan step '{id}' does not resolve to an exec command ({})",
            match other {
                Some(CommandSpec::Composite(_)) => "composite",
                Some(CommandSpec::Clone(_)) => "unmaterialized clone",
                _ => "unknown",
            }
        ),
    }
    Ok((Value::Object(step), tools))
}

/// Union `more` into `tools`, keeping first-seen order.
fn merge_tools(tools: &mut Vec<Tool>, more: Vec<Tool>) {
    for tool in more {
        if !tools.iter().any(|t| t.name == tool.name) {
            tools.push(tool);
        }
    }
}

/// Program, args, env, cwd, timeout and the tools they need, expanded
/// exactly as the dry-run preview expands them, with the dry-run's
/// env-secret redaction. Returns the tools too, for the plan-level list.
fn exec_fields(
    obj: &mut Map<String, Value>,
    e: &ExecCommandSpec,
    vars: &Variables,
) -> anyhow::Result<Vec<Tool>> {
    let program = vars.try_expand(&e.program)?.into_owned();
    let args = e
        .args
        .iter()
        .map(|a| Ok(vars.try_expand(a)?.into_owned()))
        .collect::<anyhow::Result<Vec<_>>>()?;
    let tools = exec_tools(e, &program, &args);
    obj.insert(
        "tools".into(),
        json!(tools.iter().map(|t| &t.name).collect::<Vec<_>>()),
    );
    obj.insert("program".into(), json!(program));
    obj.insert("args".into(), json!(args));
    obj.insert("display".into(), json!(e.display_cmd()));
    let mut env = Map::new();
    for (k, v) in &e.env {
        env.insert(k.clone(), json!(env_display_value(k, v, vars)?));
    }
    obj.insert("env".into(), Value::Object(env));
    let cwd = match &e.cwd {
        Some(cwd) => Some(vars.try_expand(&cwd.to_string_lossy())?.into_owned()),
        None => None,
    };
    obj.insert("cwd".into(), json!(cwd));
    obj.insert("timeoutSecs".into(), json!(e.timeout_secs));
    Ok(tools)
}

/// Where a command's spec came from: the store it resolves from, refined by
/// the loader's provenance — a `clone` (with its source and whether it
/// overrides `exclusive`) or a stack default copied in by `[extend]` — and
/// whether an `[extend.<name>]` section applied to it.
fn origin_json(runner: &CommandRunner, config: &Config, id: &str) -> Value {
    let Some((name, source)) = runner.locate(id) else {
        return json!({ "source": "unknown" });
    };
    let extended = config.extend.contains_key(name);
    if let Some(clone) = config.provenance.clones.get(name) {
        return json!({
            "source": "clone",
            "cloneOf": clone.source,
            "exclusiveOverridden": clone.exclusive_overridden,
            "extended": extended,
        });
    }
    let source = match source {
        CommandSource::Config
            if config
                .provenance
                .extended_stack_defaults
                .iter()
                .any(|n| n == name) =>
        {
            "stack"
        }
        CommandSource::Config => "config",
        CommandSource::Stack => "stack",
        CommandSource::Extension => "extension",
        CommandSource::Builtin => "builtin",
    };
    json!({ "source": source, "extended": extended })
}

/// Human-readable rendering: each named command's tree with its stages.
/// Per-step program/args/env detail is what `--json` (or `--dry-run`) is
/// for; this view answers "what runs together, and in what order".
///
/// # Errors
///
/// Writing `w` fails.
pub fn write_text(
    runner: &CommandRunner,
    plans: &[NamePlan],
    w: &mut dyn Write,
) -> anyhow::Result<()> {
    for plan in plans {
        writeln!(w, "{}", audit_safe(&plan.name))?;
        write_node(runner, &plan.plan, 1, w)?;
    }
    Ok(())
}

fn write_node(
    runner: &CommandRunner,
    plan: &CommandPlan,
    depth: usize,
    w: &mut dyn Write,
) -> anyhow::Result<()> {
    let indent = "  ".repeat(depth);
    match plan {
        CommandPlan::Stage {
            leaf_ids,
            parallel,
            fail_fast,
        } => {
            writeln!(
                w,
                "{indent}{} (fail_fast = {fail_fast})",
                if *parallel { "parallel" } else { "sequential" }
            )?;
            for (i, batch) in stages(runner, leaf_ids, *parallel).iter().enumerate() {
                let label = if batch.len() > 1 {
                    "concurrent"
                } else if batch.iter().any(|id| *parallel && is_exclusive(runner, id)) {
                    "exclusive"
                } else {
                    "alone"
                };
                let steps: Vec<String> = batch.iter().map(|id| audit_safe(id)).collect();
                writeln!(
                    w,
                    "{indent}  stage {} [{label}]: {}",
                    i.saturating_add(1),
                    steps.join(", ")
                )?;
            }
        }
        CommandPlan::Sequence {
            children,
            fail_fast,
        } => {
            writeln!(w, "{indent}sequence (fail_fast = {fail_fast})")?;
            for child in children {
                write_node(runner, child, depth.saturating_add(1), w)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::super::ExplainOutput;

    fn explain(names: &[&str], json: bool) -> String {
        let output = if json {
            ExplainOutput::Json
        } else {
            ExplainOutput::Text
        };
        explain_as(names, output)
    }

    fn explain_as(names: &[&str], output: ExplainOutput) -> String {
        let config = Arc::new(ops_core::config::load_config_or_default("test-explain"));
        let names: Vec<String> = names.iter().map(ToString::to_string).collect();
        let mut out = Vec::new();
        super::super::run_explain_to(config, &names, output, &mut out).expect("explain");
        String::from_utf8(out).expect("utf8")
    }

    /// TASK-2280: `ops explain` never executes a step, for any command —
    /// including the hook commands. Every exec here touches a marker, so a
    /// single spawn anywhere leaves evidence.
    #[test]
    #[serial_test::serial]
    fn explain_executes_no_step_for_any_command() {
        let (dir, _guard) = crate::test_utils::with_temp_config(
            r#"
[commands.touch-a]
program = "touch"
args = ["ran-marker"]
exclusive = true

[commands.touch-b]
program = "touch"
args = ["ran-marker"]

[commands.group]
commands = ["touch-a", "touch-b"]
parallel = true

[commands.run-before-commit]
commands = ["group", "touch-b"]

[commands.run-before-push]
commands = ["touch-a"]
"#,
        );
        for names in [
            ["run-before-commit"].as_slice(),
            ["run-before-push"].as_slice(),
            ["touch-a"].as_slice(),
            ["group", "run-before-commit", "run-before-push"].as_slice(),
        ] {
            explain(names, true);
            explain(names, false);
        }
        assert!(
            !dir.path().join("ran-marker").exists(),
            "ops explain must never run a step"
        );
    }

    /// The JSON document carries the schema version, each composite's
    /// flags, the exclusive-split stages, each step's resolved fields, and
    /// the origin of cloned, extended and stack-default commands.
    #[test]
    #[serial_test::serial]
    fn explain_json_reports_stages_flags_env_and_origin() {
        let (dir, _guard) = crate::test_utils::with_temp_config(
            r#"
[commands.fmt-a]
program = "echo"
args = ["a"]
exclusive = true
env = { PLAIN = "value", API_TOKEN = "hunter2" }

[commands.lint-b]
program = "echo"
args = ["b"]
cwd = "sub"

[commands.lint-c]
clone = "lint-b"

[commands.gate]
commands = ["fmt-a", "lint-b", "lint-c", "build"]
parallel = true
fail_fast = false

[extend.build]
args = ["--locked"]
"#,
        );
        std::fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"x\"\n")
            .expect("Cargo.toml");
        let doc: serde_json::Value =
            serde_json::from_str(&explain(&["gate"], true)).expect("valid JSON");

        assert_eq!(doc["schemaVersion"], super::SCHEMA_VERSION);
        assert_eq!(doc["kind"], "command-plan");
        let plan = &doc["commands"][0]["plan"];
        assert_eq!(doc["commands"][0]["name"], "gate");
        assert_eq!(plan["type"], "stage");
        assert_eq!(plan["parallel"], true);
        assert_eq!(plan["failFast"], false);
        assert_eq!(
            plan["stages"],
            serde_json::json!([
                { "concurrent": false, "steps": ["fmt-a"] },
                { "concurrent": true, "steps": ["lint-b", "lint-c", "build"] },
            ])
        );
        assert_eq!(doc["composites"][0]["name"], "gate");
        assert_eq!(doc["composites"][0]["origin"]["source"], "config");

        let step = |id: &str| {
            doc["steps"]
                .as_array()
                .and_then(|s| s.iter().find(|s| s["id"] == id))
                .unwrap_or_else(|| panic!("step {id} missing from {doc}"))
                .clone()
        };
        let fmt_a = step("fmt-a");
        assert_eq!(fmt_a["exclusive"], true);
        assert_eq!(fmt_a["program"], "echo");
        assert_eq!(fmt_a["args"], serde_json::json!(["a"]));
        assert_eq!(fmt_a["env"]["PLAIN"], "value");
        assert_eq!(fmt_a["env"]["API_TOKEN"], "***REDACTED***");
        assert_eq!(step("lint-b")["cwd"], "sub");
        assert_eq!(
            step("lint-c")["origin"],
            serde_json::json!({
                "source": "clone",
                "cloneOf": "lint-b",
                "exclusiveOverridden": false,
                "extended": false,
            })
        );
        let build = step("build");
        assert_eq!(
            build["origin"],
            serde_json::json!({ "source": "stack", "extended": true })
        );
        assert!(
            build["args"]
                .as_array()
                .is_some_and(|a| a.iter().any(|v| v == "--locked")),
            "the extended args must show up: {build}"
        );
    }

    /// TASK-2326: `tools` lists each binary the plan needs once, in
    /// first-use order, with the steps that need it — cargo plugins and the
    /// tools `ops` builtins spawn included, `ops` itself and path programs
    /// excluded.
    #[test]
    #[serial_test::serial]
    fn explain_json_lists_required_tools() {
        let (_dir, _guard) = crate::test_utils::with_temp_config(
            r#"
[commands.nt]
program = "cargo"
args = ["nextest", "run"]

[commands.lint]
program = "cargo"
args = ["clippy"]

[commands.bld]
program = "cargo"
args = ["build"]

[commands.script]
program = "./scripts/check.sh"

[commands.gate]
commands = ["nt", "lint", "bld", "script", "sec", "end-of-file-fixer"]
"#,
        );
        let doc: serde_json::Value =
            serde_json::from_str(&explain(&["gate"], true)).expect("valid JSON");
        let names: Vec<&str> = doc["tools"]
            .as_array()
            .expect("tools array")
            .iter()
            .filter_map(|t| t["name"].as_str())
            .collect();
        assert_eq!(
            names,
            ["cargo", "cargo-nextest", "cargo-clippy", "trivy"],
            "{doc}"
        );
        assert_eq!(
            doc["tools"][0]["requiredBy"],
            serde_json::json!(["nt", "lint", "bld"])
        );
        assert_eq!(doc["tools"][3]["requiredBy"], serde_json::json!(["sec"]));
        assert_eq!(doc["tools"][3]["optional"], false);
        assert_eq!(doc["tools"][2]["install"], "rustup component add clippy");
        assert!(
            doc["tools"]
                .as_array()
                .is_some_and(|t| t.iter().all(|t| t.get("installedVersion").is_none())),
            "plain --json probes nothing, so no tool carries a version: {doc}"
        );
        let step = |id: &str| {
            doc["steps"]
                .as_array()
                .and_then(|s| s.iter().find(|s| s["id"] == id))
                .unwrap_or_else(|| panic!("step {id} missing from {doc}"))["tools"]
                .clone()
        };
        assert_eq!(step("nt"), serde_json::json!(["cargo", "cargo-nextest"]));
        assert_eq!(step("script"), serde_json::json!([]));
        assert_eq!(step("end-of-file-fixer"), serde_json::json!([]));
    }

    /// TASK-2335: `--tool-versions` adds each tool's `installedVersion` —
    /// the `--version` banner when the tool runs, `null` when it is missing.
    #[test]
    #[serial_test::serial]
    fn explain_tool_versions_reports_installed_versions() {
        let (_dir, _guard) = crate::test_utils::with_temp_config(
            r#"
[commands.bld]
program = "cargo"
args = ["build"]

[commands.missing]
program = "ops-no-such-tool-2335"

[commands.gate]
commands = ["bld", "missing"]
"#,
        );
        let doc: serde_json::Value =
            serde_json::from_str(&explain_as(&["gate"], ExplainOutput::JsonWithToolVersions))
                .expect("valid JSON");
        let version = |name: &str| {
            doc["tools"]
                .as_array()
                .and_then(|t| t.iter().find(|t| t["name"] == name))
                .unwrap_or_else(|| panic!("tool {name} missing from {doc}"))["installedVersion"]
                .clone()
        };
        assert!(
            version("cargo")
                .as_str()
                .is_some_and(|v| v.starts_with("cargo ")),
            "{doc}"
        );
        assert_eq!(version("ops-no-such-tool-2335"), serde_json::Value::Null);
    }

    /// A sequential composite is a sequence node whose entries keep their
    /// own schedules, and the text view names every stage.
    #[test]
    #[serial_test::serial]
    fn explain_renders_sequences_in_json_and_text() {
        let (_dir, _guard) = crate::test_utils::with_temp_config(
            r#"
[commands.one]
program = "echo"
args = ["1"]

[commands.two]
program = "echo"
args = ["2"]

[commands.pair]
commands = ["one", "two"]
parallel = true

[commands.seq]
commands = ["pair", "one"]
"#,
        );
        let doc: serde_json::Value =
            serde_json::from_str(&explain(&["seq"], true)).expect("valid JSON");
        let plan = &doc["commands"][0]["plan"];
        assert_eq!(plan["type"], "sequence");
        assert_eq!(plan["children"][0]["type"], "stage");
        assert_eq!(plan["children"][0]["parallel"], true);
        assert_eq!(plan["children"][1]["steps"], serde_json::json!(["one"]));

        let text = explain(&["seq"], false);
        assert!(text.contains("sequence (fail_fast = true)"), "{text}");
        assert!(text.contains("stage 1 [concurrent]: one, two"), "{text}");
    }

    /// An unknown command fails the whole explain rather than printing a
    /// partial plan.
    #[test]
    #[serial_test::serial]
    fn explain_unknown_command_errors() {
        let (_dir, _guard) = crate::test_utils::with_temp_config("");
        let config = Arc::new(ops_core::config::load_config_or_default("test-explain"));
        let err = super::super::run_explain_to(
            config,
            &["definitely-not-a-command".to_string()],
            ExplainOutput::Json,
            &mut Vec::new(),
        )
        .expect_err("unknown command must fail");
        assert!(format!("{err:#}").contains("definitely-not-a-command"));
    }
}
