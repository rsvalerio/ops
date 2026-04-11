---
id: TASK-005
title: CommandId is a bare String type alias — no compile-time safety
status: To Do
assignee: []
created_date: '2026-04-07 00:00:00'
updated_date: '2026-04-07 22:48'
labels:
  - rust-code-quality
  - CQ
  - API-2
  - low
  - effort-M
  - crate-core
dependencies: []
ordinal: 4000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Location**: `crates/core/src/config/mod.rs:265`
**Anchor**: `type CommandId`
**Impact**: `pub type CommandId = String;` provides semantic clarity but no compile-time type safety. A `String` command ID can be accidentally swapped with any other `String` parameter (display labels, program names, error messages) without compiler error. The code comments acknowledge this: "could be changed to a newtype for stronger type safety if needed."

**Notes**:
Per API-2, wrap primitives for type safety when the value has domain meaning and is passed alongside other strings. `CommandId` is used in `IndexMap<CommandId, CommandSpec>`, event structs (`RunnerEvent`), `StepResult`, and function signatures throughout runner and CLI crates.

A newtype would be: `pub struct CommandId(pub String);` with `Deref<Target=str>`, `Display`, `From<String>`, `From<&str>`, and serde support. The `effort-M` rating reflects the widespread usage across 4 crates, though most call sites would only need minor changes due to `Deref`.

Trade-off: the newtype adds ceremony for `.0` access or `.as_str()` calls. Current usage is consistent and well-scoped, so the risk of misuse is low. This is a strengthening opportunity, not a bug.
<!-- SECTION:DESCRIPTION:END -->
