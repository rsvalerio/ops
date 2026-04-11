---
id: TASK-014
title: "run_command_dry_run_to has 5-level nesting in env display"
status: To Do
assignee: []
created_date: '2026-04-07 12:00:00'
labels: [rust-code-quality, CQ, FN-2, low, effort-S, crate-cli]
dependencies: []
---

## Description

**Location**: `crates/cli/src/run_cmd.rs:171-203`
**Anchor**: `fn run_command_dry_run_to`
**Impact**: The for-loop over leaf IDs contains a match on CommandSpec, then conditionals for args/env/cwd/timeout, with the env block reaching 5 levels deep (for → match → arm → if → for).

**Notes**:
Extract a helper `fn print_exec_spec(w: &mut dyn Write, spec: &ExecCommandSpec) -> anyhow::Result<()>` that handles printing program, args, env, cwd, and timeout for a single exec spec. This flattens the match arm body and keeps the outer loop at 2 levels.
