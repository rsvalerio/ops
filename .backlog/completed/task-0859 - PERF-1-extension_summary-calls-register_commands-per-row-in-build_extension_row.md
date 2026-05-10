---
id: TASK-0859
title: >-
  PERF-1: extension_summary calls register_commands per-row in
  build_extension_row
status: Done
assignee: []
created_date: '2026-05-02 09:19'
updated_date: '2026-05-02 14:38'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/extension_cmd.rs:121-138`

**What**: When an extension does not expose a static command_names, the fallback path constructs a fresh CommandRegistry, calls ext.register_commands, drains duplicates, then collects keys. This is invoked from build_extension_row once per row of the table in write_extension_table, so a list of N legacy extensions performs N independent register_commands calls and N independent registry allocations.

**Why it matters**: register_commands is documented as potentially I/O-heavy (PERF-1 / TASK-0513 acknowledges this). Even though TASK-0513 added the static fast path, the slow fallback is still latency-amplified by the per-row call site and runs serially under the interactive command.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Hoist the extension_summary calls out of the per-row loop into a single map computed before write_extension_table iterates
- [x] #2 Verify the duplicate-insert warning is still emitted exactly once per offending extension regardless of how many table rows render it
- [x] #3 No observable change to rendered table content under the existing run_extension_list_outputs_extensions test
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Hoisted extension_summary out of the per-row loop in write_extension_table: a single Vec<(types, commands)> is computed once per render pass, then build_extension_row receives the precomputed summary. The duplicate-insert warning still fires inside extension_summary which is invoked exactly once per extension. run_extension_list_outputs_extensions and extension_summary_warns_on_self_shadow both pass; ops verify clean.
<!-- SECTION:NOTES:END -->
