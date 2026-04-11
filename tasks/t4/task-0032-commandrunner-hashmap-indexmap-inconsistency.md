---
id: TASK-0032
title: "CommandRunner mixes HashMap and IndexMap for command sources"
status: Triage
assignee: []
created_date: '2026-04-09 20:35:00'
labels: [rust-code-quality, CQ, READ-6, low, crate-runner]
dependencies: []
---

## Description

**Location**: `crates/runner/src/command/mod.rs:76-84`
**Anchor**: `struct CommandRunner`
**Impact**: `config.commands` (via `Config`) and `stack_commands` use `IndexMap` (deterministic insertion order), but `extension_commands` uses `std::collections::HashMap` (non-deterministic iteration order). This inconsistency means `canonical_id()` and `resolve_alias()` may return different results across runs when multiple extensions register commands with overlapping aliases.

**Notes**:
In `canonical_id()` (line 176-197), the function iterates `stack_commands` then `extension_commands` to find alias matches. With `HashMap`, if two extensions register commands with the same alias, which one is found first is non-deterministic.

Similarly, `resolve_alias()` (line 200-214) iterates `extension_commands.values()` with non-deterministic order.

`list_command_ids()` (line 217-224) mitigates this by sorting the result, but the underlying iteration is still non-deterministic.

Fix: change `extension_commands` from `std::collections::HashMap` to `IndexMap` to match the other command sources. This ensures deterministic resolution order regardless of how many extensions are loaded.
