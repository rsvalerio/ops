---
id: TASK-0029
title: "Extract shared execution lifecycle in run_commands and run_command_cli"
status: Triage
assignee: []
created_date: '2026-04-09 20:30:00'
labels: [rust-code-duplication, CD, DUP-1, DUP-5, medium, crate-cli]
dependencies: [TASK-0011]
---

## Description

**Location**: `crates/cli/src/run_cmd.rs:54-75` and `crates/cli/src/run_cmd.rs:217-235`
**Anchor**: `fn run_commands`, `fn run_command_cli`
**Impact**: ~8 near-identical lines covering the execution lifecycle appear in both functions. A change to the display/runtime/logging pattern (e.g., switching to a persistent runtime, adding structured logging, or changing echo guard behavior) must be applied in both places.

**Notes**:
Duplicated block in `run_commands` (lines 54-75):
```rust
let display_map = build_display_map(&runner, &all_leaf_ids);
let mut display = ProgressDisplay::new(runner.output_config(), display_map, &runner.config().themes)?;
let _echo_guard = EchoGuard::disable_echo();
let rt = tokio::runtime::Runtime::new()?;
let results: Vec<StepResult> = rt.block_on(async { runner.run_plan(&all_leaf_ids, &mut |event| display.handle_event(event)).await });
drop(_echo_guard);
log_step_results(&results);
let success = results.iter().all(|r| r.success);
```

Near-identical block in `run_command_cli` (lines 217-235):
```rust
let display_map = build_display_map(runner, &leaf_ids);
let mut display = ProgressDisplay::new(runner.output_config(), display_map, &runner.config().themes)?;
let _echo_guard = EchoGuard::disable_echo();
let rt = tokio::runtime::Runtime::new()?;
let results: Vec<StepResult> = rt.block_on(async { runner.run(name, &mut |event| display.handle_event(event)).await })?;
drop(_echo_guard);
log_step_results(&results);
let success = results.iter().all(|r| r.success);
```

The only difference is the runner method called (`run_plan` vs `run`). Combined with TASK-0011 (setup duplication in the same functions), a single extracted helper could cover both setup and execution:

```rust
fn execute_with_display(
    runner: &mut CommandRunner,
    run_fn: impl FnOnce(&mut CommandRunner, &mut impl FnMut(RunnerEvent)) -> ...
) -> anyhow::Result<bool> { ... }
```
