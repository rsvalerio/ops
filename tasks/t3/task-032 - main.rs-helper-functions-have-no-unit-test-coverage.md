---
id: TASK-032
title: main.rs helper functions have no unit test coverage
status: To Do
assignee: []
created_date: '2026-04-08 00:00:00'
labels: [rust-test-quality, TQ, TEST-5, medium, effort-S, crate-cli]
dependencies: []
---

## Description

**Location**: `crates/cli/src/main.rs:181-255`
**Anchor**: `fn is_toplevel_help`, `fn inject_dynamic_commands`
**Impact**: Two non-trivial helper functions in the CLI entry point have zero unit test coverage. `is_toplevel_help` has multi-branch logic (iterates args, distinguishes subcommand help from top-level help) and `inject_dynamic_commands` builds dynamic clap subcommands from config and stack defaults with deduplication logic. Both are only exercised indirectly via integration tests (`cli_help`), which asserts broad output but does not verify edge cases like mixed flags-before-positional, empty configs, or duplicate command names.

**Notes**:
Add a `#[cfg(test)] mod tests` block in `main.rs` with unit tests:

For `is_toplevel_help`:
- `ops -h` → true
- `ops --help` → true
- `ops build -h` → false (positional before help flag)
- `ops -d --help` → true (flags only before help)
- `ops` → false (no help flag)

For `inject_dynamic_commands`:
- Empty config → no subcommands added
- Config commands added, builtins skipped
- Stack defaults added, duplicates with config skipped
- Help text from spec.help() vs display_cmd_fallback()
