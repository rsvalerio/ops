---
id: TASK-0016
title: "run_command_dry_run_to has 7-level nesting in env-render loop"
status: Triage
assignee: []
created_date: '2026-04-09 19:25:00'
labels: [rust-code-quality, CQ, FN-2, high, crate-cli]
dependencies: []
---

## Description

**Location**: `crates/cli/src/run_cmd.rs:171-203`
**Anchor**: `fn run_command_dry_run_to`
**Impact**: Lines 179-188 reach 7 levels of nesting: function → for → match → Some(Exec) arm → if env not empty → for (k,v) → if is_sensitive. This is the deepest nesting in the codebase. The env-rendering block could be extracted with no loss of clarity.

**Notes**:
Extract `fn render_env_vars(w: &mut dyn Write, env: &IndexMap<String, String>) -> io::Result<()>` as a standalone helper. This removes 3 levels of nesting from the main function and makes the env display logic independently testable.
