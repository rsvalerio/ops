---
id: TASK-0008
title: 'CommandId is a bare type alias, not a newtype (API-2)'
status: Done
assignee: []
created_date: '2026-04-10 16:00:00'
updated_date: '2026-04-11 10:06'
labels:
  - rust-code-quality
  - CQ
  - API-2
  - low
  - crate-core
dependencies: []
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Location**: `crates/core/src/config/mod.rs:265`
**Anchor**: `type CommandId`
**Impact**: `pub type CommandId = String;` provides no type safety — any `String` can be passed where a `CommandId` is expected, and a `CommandId` can silently become a display string or error message. This defeats the purpose of having a named type.

**Notes**:
`CommandId` is used across 20+ function signatures in `crates/runner` and `crates/cli` (e.g., `resolve(id: &str)`, `run_plan(command_ids: &[CommandId])`, `expand_to_leaves(id: &str)`). A newtype `pub struct CommandId(String)` would prevent accidental mixing with other strings and enable domain methods (e.g., `CommandId::new()`, `AsRef<str>`).

Trade-off: this is a wide-impact refactor. The type alias is used pervasively, and many functions accept `&str` rather than `&CommandId`, so the benefit is partially lost at call sites. A phased approach (introduce newtype, migrate incrementally) would minimize risk. Alternatively, if the team considers the current alias sufficient for documentation purposes, this finding can be closed.

API-2: "Newtype pattern: wrap primitives for type safety to prevent argument order mistakes; zero-cost abstraction."
<!-- SECTION:DESCRIPTION:END -->
