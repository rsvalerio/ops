---
id: TASK-2078
title: 'FN-3: exec_command and exec_command_raw thread 6-7 positional params that ExecTaskCtx already bundles'
status: Done
assignee: []
created_date: '2026-09-07 22:57'
updated_date: '2026-09-09 18:33'
labels:
  - code-review-rust
  - structure
dependencies: []
parent_task_id: 'TASK-2246'
modified_files:
  - crates/runner/src/command/exec.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/exec.rs:568-576,664-671`

**What**: `exec_command` takes 7 positional parameters (`id, spec, workspace_cache, cwd, vars, policy, emit`) and `exec_command_raw` takes 6, both behind bare `#[allow(clippy::too_many_arguments)]` with no adjacent arity rationale (the doc comments above them discuss retries and stdio, not the parameter count). The parallel path already solved this exact shape: `ExecTaskCtx` bundles `cwd / vars / tx / abort / policy / workspace_cache` into one Clone bag (FN-9 / TASK-0778), and `ExecTaskCtx::new` carries its own `too_many_arguments` allow for the same reason.

**Why it matters**: FN-3 — five same-typed-ish handles threaded positionally is the permutation bug the rule exists for (e.g. swapping `cwd: &Arc<PathBuf>` and `vars: &Arc<Variables>` compiles nowhere, but a future `Arc<A>`/`Arc<B>` pair with compatible `AsRef` could). Two call sites already diverge: the sequential path (`run_exec` → `exec_command`) and the raw path thread the handles by hand while the parallel path uses the bag, so every new runner-scoped handle (the next `workspace_cache`-style addition) must be added in three signatures instead of one struct. The two bare allows also sit below the project's own bar in docs/clippy.md ("an exception is written down" next to the code).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The shared runner-scoped handles (workspace_cache, cwd, vars, policy) reach exec_command / exec_command_raw through a single grouped parameter (extend ExecTaskCtx with an emit-sink variant, or introduce the equivalent bag for the sequential/raw paths)
- [x] #2 The remaining signatures are at or under clippy's threshold, or every retained too_many_arguments allow carries an adjacent reason per docs/clippy.md
- [x] #3 Sequential, raw, and parallel spawn paths share one bag type, so adding a handle touches one struct

<!-- AC:END -->
