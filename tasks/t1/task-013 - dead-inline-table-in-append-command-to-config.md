---
id: TASK-013
title: "Dead cmd_table InlineTable in append_command_to_config"
status: To Do
assignee: []
created_date: '2026-04-07 12:00:00'
labels: [rust-code-quality, CQ, LINT-8, medium, effort-S, crate-cli]
dependencies: []
---

## Description

**Location**: `crates/cli/src/new_command_cmd.rs:95-105`
**Anchor**: `fn append_command_to_config`
**Impact**: Lines 95-105 build a `cmd_table` (InlineTable) with program and args, but it is never used — line 118 inserts `cmd` (a regular Table built at lines 108-116) instead. This is dead code that also duplicates the array construction (lines 98-101 vs 111-115).

**Notes**:
Remove lines 95-105 entirely. The regular Table at lines 108-116 is what gets inserted. The duplicate array construction will disappear with the removal.
