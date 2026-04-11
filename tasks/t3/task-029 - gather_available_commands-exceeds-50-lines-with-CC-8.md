---
id: TASK-029
title: "gather_available_commands exceeds 50 lines with CC≈8"
status: To Do
assignee: []
created_date: '2026-04-08 00:00:00'
labels: [rust-code-quality, CQ, FN-1, FN-6, medium, effort-S, crate-cli]
dependencies: []
---

## Description

**Location**: `crates/cli/src/run_before_commit_cmd.rs:20-72`
**Anchor**: `fn gather_available_commands`
**Impact**: gather_available_commands is 53 lines with cyclomatic complexity ≈8. It handles three priority levels of command sources (config commands, stack default commands, extension commands) with deduplication logic in a single function, mixing collection, filtering, and deduplication concerns.

**Notes**:
The function collects commands from three sources with a `seen` HashSet for deduplication. Refactoring: extract per-source collection into helpers (`gather_config_commands()`, `gather_stack_commands()`, `gather_extension_commands()`) that each return a filtered iterator, then merge with deduplication at the call site.
