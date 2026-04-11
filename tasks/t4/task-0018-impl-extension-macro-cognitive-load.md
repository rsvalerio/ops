---
id: TASK-0018
title: "impl_extension! macro is 140 lines with high cognitive load"
status: Triage
assignee: []
created_date: '2026-04-09 19:25:00'
labels: [rust-code-quality, CQ, CL-1, CL-5, medium, crate-extension]
dependencies: []
---

## Description

**Location**: `crates/extension/src/lib.rs:407-546`
**Anchor**: `macro impl_extension`
**Impact**: The macro has 4 match arms distinguished by optional trailing elements (`factory:`, `register_commands:`). Each arm also contains optional `$(stack:)?` and `$(command_names:)?` fragments, creating a cross-product of combinations. A reader must mentally expand all combinations to verify correctness. The `factory:` arm silently emits a `#[linkme::distributed_slice]` static — a global side effect that is surprising from what looks like an `impl` block. The internal `@accessors` rule is not part of the public API but is invoked inside each arm, adding indirection.

**Notes**:
Consider: (1) documenting the 4 arm variants with examples at the macro's doc comment, (2) extracting `@accessors` into a separate helper macro with its own doc comment, (3) adding a comment on the `factory:` arm explaining the distributed_slice global registration side effect. If the macro grows further, consider replacing some arms with a builder-pattern struct approach instead.
