---
id: TASK-0021
title: 'Sequential run_plan always stops on first failure, ignoring fail_fast'
status: Done
assignee: []
created_date: '2026-04-10 23:30:00'
updated_date: '2026-04-11 09:55'
labels:
  - rust-code-quality
  - CQ
  - FN-9
  - READ-5
  - medium
  - crate-runner
dependencies: []
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Location**: `crates/runner/src/command/mod.rs:319-337`
**Anchor**: `fn run_plan`, `fn run`
**Impact**: `run_plan` (sequential execution) always breaks on first failure (`if should_stop { break; }` at line 330-332), with `should_stop` set to `!r.success` in `execute_step` (line 307). The `CompositeCommandSpec.fail_fast` field (config/mod.rs:272) is only honored in the parallel path — `run` (line 491-496) passes `c.fail_fast` to `run_plan_parallel` but the sequential path (`_ => self.run_plan(...)`) has no `fail_fast` parameter.

This means a composite command configured with `fail_fast = false` and `parallel = false` will still stop on the first failure, contradicting the user's intent. The config documents `fail_fast` as "When true (default), stop remaining steps on first failure. When false, run all steps" — this contract is only honored for parallel execution.

**Notes**:
FN-9: "Explicit dependencies; no implicit state." READ-5: "Make invariants explicit."

The `run_plan` doc comment says "stop on first failure" — so the behavior is documented, but the asymmetry with the parallel path is surprising.

Fix: Add a `fail_fast: bool` parameter to `run_plan`:
```rust
pub async fn run_plan(
    &self,
    command_ids: &[CommandId],
    fail_fast: bool,
    on_event: &mut impl FnMut(RunnerEvent),
) -> Vec<StepResult> {
    // ...
    for id in command_ids {
        let (result, should_stop) = self.execute_step(id, on_event).await;
        results.push(result);
        if fail_fast && should_stop {
            break;
        }
    }
    // ...
}
```

Then update `run` to pass `c.fail_fast` for both paths:
```rust
CommandSpec::Composite(c) if c.parallel => {
    self.run_plan_parallel(&plan, c.fail_fast, on_event).await
}
CommandSpec::Composite(c) => {
    self.run_plan(&plan, c.fail_fast, on_event).await
}
_ => self.run_plan(&plan, true, on_event).await,  // exec: fail_fast by default
```

Note: `run_commands` in `run_cmd.rs` calls `run_plan` directly (line 68) — it would need to pass `true` to preserve current behavior.
<!-- SECTION:DESCRIPTION:END -->
