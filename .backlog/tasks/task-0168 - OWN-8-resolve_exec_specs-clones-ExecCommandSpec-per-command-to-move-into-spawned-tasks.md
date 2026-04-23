---
id: TASK-0168
title: >-
  OWN-8: resolve_exec_specs clones ExecCommandSpec per command to move into
  spawned tasks
status: To Do
assignee: []
created_date: '2026-04-22 21:24'
updated_date: '2026-04-23 06:45'
labels:
  - rust-code-review
  - OWN
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/mod.rs:424-438` (resolve_exec_specs) and `command/mod.rs:500-510` (spawn_parallel_tasks).

**What**: `resolve_exec_specs` clones every `ExecCommandSpec` with a documented rationale "Clone is required: specs must be owned to move into spawned tasks." Spec contains `String` program, `Vec<String> args`, `IndexMap<String,String> env` — 2N heap allocations per parallel plan element (N strings + env map). The current comment rationalizes this but the ownership design is avoidable: wrap the spec in an `Arc<ExecCommandSpec>` stored in `Config.commands` and clone the `Arc` (cheap refcount bump) instead of deep-cloning Strings and the env IndexMap.

**Why it matters**: OWN-8 ("cloning to satisfy the borrow checker/move semantics → rethink ownership design"). In practice a 10-step parallel plan with 5-6 env vars each clones ~60 small heap strings. Low-to-medium perf impact but the design fix (Arc-wrap at config-load time) is mechanical and makes the intent explicit. See also PERF-3.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Consider Arc<ExecCommandSpec> in Config.commands so parallel tasks clone the Arc
<!-- AC:END -->
