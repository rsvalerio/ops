---
id: TASK-0022
title: "CommandId is a type alias for String — should be a newtype"
status: Triage
assignee: []
created_date: '2026-04-09 19:25:00'
labels: [rust-code-quality, CQ, API-2, low, crate-core]
dependencies: []
---

## Description

**Location**: `crates/core/src/config/mod.rs:292`
**Anchor**: `type CommandId`
**Impact**: `type CommandId = String` provides no compile-time protection against passing an arbitrary `String` where a validated command ID is expected. Functions like `exec_standalone` take both `id: CommandId` and `spec: ExecCommandSpec` — since `CommandId` is just `String`, the compiler cannot catch argument transpositions or invalid command names. The `CommandRegistry` (`IndexMap<CommandId, CommandSpec>`) accepts any string as a key.

**Notes**:
Introduce `struct CommandId(Arc<str>)` or `struct CommandId(String)` with a validated constructor. This is zero-cost at runtime and prevents a class of footguns. The existing comment at line 292 already acknowledges this could be a newtype. Low severity because current usage is correct and the codebase is small enough that misuse is caught in review.
