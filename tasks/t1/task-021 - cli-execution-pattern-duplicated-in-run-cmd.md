---
id: TASK-021
title: "CLI execution pattern (display + runtime + results) duplicated in run_cmd.rs"
status: To Do
assignee: []
created_date: '2026-04-08 00:00:00'
labels: [rust-code-duplication, CD, DUP-1, DUP-2, DUP-5, medium, effort-M, crate-cli]
dependencies: [TASK-011]
---

## Description

**Location**: `crates/cli/src/run_cmd.rs:54-75` and `crates/cli/src/run_cmd.rs:217-235`
**Anchor**: `fn run_commands`, `fn run_command_cli`
**Impact**: The CLI execution pattern — build display map, create ProgressDisplay, disable echo, create tokio runtime, block on async execution, drop echo guard, log results, check success — is duplicated between the multi-command and single-command paths. This is separate from the setup block duplication covered in TASK-011.

**Notes**:
`run_commands` (lines 54-75):
```rust
let display_map = build_display_map(&runner, &all_leaf_ids);
let mut display =
    ProgressDisplay::new(runner.output_config(), display_map, &runner.config().themes)?;
let _echo_guard = EchoGuard::disable_echo();
let rt = tokio::runtime::Runtime::new()?;
let results: Vec<StepResult> = rt.block_on(async { runner.run_plan(...).await });
drop(_echo_guard);
log_step_results(&results);
let success = results.iter().all(|r| r.success);
```

`run_command_cli` (lines 217-235):
```rust
let display_map = build_display_map(runner, &leaf_ids);
let mut display =
    ProgressDisplay::new(runner.output_config(), display_map, &runner.config().themes)?;
let _echo_guard = EchoGuard::disable_echo();
let rt = tokio::runtime::Runtime::new()?;
let results: Vec<StepResult> = rt.block_on(async { runner.run(...).await? });
drop(_echo_guard);
log_step_results(&results);
let success = results.iter().all(|r| r.success);
```

The only difference is the async call (`run_plan` vs `run`) and the leaf_ids source. Combined with TASK-011 (setup duplication), the two functions share ~15 lines of boilerplate each.

Fix: extract a helper that takes the runner, leaf_ids (or command name), and an async execution closure:
```rust
fn execute_with_display(
    runner: &CommandRunner,
    leaf_ids: &[String],
    execute: impl FnOnce(&CommandRunner, &mut impl FnMut(RunnerEvent)) -> Vec<StepResult>,
) -> anyhow::Result<bool> { ... }
```
Or unify the two paths after the setup block from TASK-011 is extracted, since both ultimately build a display and run a plan.
