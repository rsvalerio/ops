---
id: TASK-030
title: "extension_cmd.rs two functions exceed 50-line threshold"
status: To Do
assignee: []
created_date: '2026-04-08 00:00:00'
labels: [rust-code-quality, CQ, FN-1, low, effort-S, crate-cli]
dependencies: []
---

## Description

**Location**: `crates/cli/src/extension_cmd.rs:146-256`
**Anchor**: `fn run_extension_show_with_tty_check`, `fn print_extension_details`
**Impact**: run_extension_show_with_tty_check (51 lines, lines 146-196) and print_extension_details (59 lines, lines 198-256) both exceed the FN-1 threshold. run_extension_show_with_tty_check mixes name resolution, interactive selection, and lookup with a complex nested Option/if-let chain. print_extension_details mixes type checks and registry lookups for multiple extension facets.

**Notes**:
Both functions share type-checking logic that could be extracted into a `format_extension_types()` helper (also flagged as DUP-004 in extension_cmd.rs lines 105-110 vs 209-215). Extracting the interactive selection and type formatting would bring both functions under threshold.
