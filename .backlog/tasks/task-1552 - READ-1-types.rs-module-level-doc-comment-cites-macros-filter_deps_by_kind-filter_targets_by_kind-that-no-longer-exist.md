---
id: TASK-1552
title: >-
  READ-1: types.rs module-level doc comment cites macros (filter_deps_by_kind!,
  filter_targets_by_kind!) that no longer exist
status: To Do
assignee:
  - TASK-1576
created_date: '2026-05-19 15:27'
updated_date: '2026-05-19 16:46'
labels:
  - code-review-rust
  - READ
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/types.rs:1-10`

**What**: The module-level doc comment claims:

> # Code Generation (DUP-STR-001, DUP-STR-002)
>
> The `filter_deps_by_kind!` and `filter_targets_by_kind!` macros reduce boilerplate
> for dependency and target accessor methods. Each macro generates multiple methods
> that differ only by the filter predicate (enum variant or target kind string).

But there are no such `macro_rules!` declarations in the file. The actual implementation uses an inherent method `Package::filter_deps_by_kind` (line 369) and call-site `.filter(|t| t.is_*())` for targets (lines 412-429). The doc is documenting a refactor that either never landed or got replaced. The cited rule IDs `DUP-STR-001` / `DUP-STR-002` are also not in the current rule taxonomy.

**Why it matters**: READ-1 covers documentation that misrepresents the implementation. New contributors reading the module preamble will hunt for non-existent macros, and the dead "DUP-STR" tags suggest a stale code-review identifier scheme that the wider repo no longer uses. Either reinstate the macros (the original DRY win) or rewrite the preamble to describe the current shape (`Package::filter_deps_by_kind` helper + iterator-filter chains for targets).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 types.rs module preamble describes the actual implementation (helper fn + filter chains)
- [ ] #2 Dangling DUP-STR-001 / DUP-STR-002 identifiers are removed or replaced with current rule IDs
<!-- AC:END -->
