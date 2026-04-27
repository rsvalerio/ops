---
id: TASK-0401
title: 'CL-1: deps check_tool magic-string dispatch hardcodes install package mapping'
status: To Do
assignee:
  - TASK-0421
created_date: '2026-04-26 09:52'
updated_date: '2026-04-26 10:10'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/lib.rs:92-114`

**What**: `check_tool(tool: &str, args: &[&str])` is a generic helper, but its error message hardcodes the install package by string-comparing the `tool` parameter:

```rust
if tool == "upgrade" { "cargo-edit" } else { "cargo-deny" }
```

Adding a third tool means editing this branch (silently mapping to "cargo-deny"). The mapping data and the dispatch helper are tangled.

**Why it matters**: Adding/renaming a tool produces wrong install instructions with no compile-time signal. The "generic" helper is not actually generic — it owns knowledge of the two specific callers.

**Suggested**: Pass install package name as an explicit parameter (e.g. `check_tool(tool, args, install_pkg)`) or build a small `(name, args, install_pkg)` table iterated by `ensure_tools`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 check_tool either takes the install package as a parameter or is replaced by a table-driven enumeration
- [ ] #2 Adding a new tool does not require editing an else branch with a default fallback
<!-- AC:END -->
