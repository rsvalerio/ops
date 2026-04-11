---
id: TASK-0009
title: "Unify Name+Description option structs duplicated across 3 interactive prompts"
status: Triage
assignee: []
created_date: '2026-04-09 00:00:00'
labels: [rust-code-duplication, CD, DUP-2, medium, crate-cli]
dependencies: []
---

## Description

**Location**: `crates/cli/src/extension_cmd.rs:299-308`, `crates/cli/src/run_before_commit_cmd.rs:9-18`, `crates/cli/src/theme_cmd.rs:154-165`
**Anchor**: `struct ExtensionOption`, `struct CommandOption`, `struct ThemeOption`
**Impact**: Three structurally identical structs (`name: String`, `description: String`) with near-identical `Display` impls used for interactive selection. Inconsistent separator (hyphen vs em dash in `run_before_commit_cmd.rs`) is likely unintentional.

**Notes**:
`ExtensionOption` and `CommandOption` are identical except for the separator character. `ThemeOption` adds an `is_custom` field and a `(custom)` marker in Display.

Options:
- Extract a generic `SelectOption { name, description }` with a standard Display impl; `ThemeOption` can wrap or extend it.
- Alternatively, use a shared trait for Display formatting of selectable items.
