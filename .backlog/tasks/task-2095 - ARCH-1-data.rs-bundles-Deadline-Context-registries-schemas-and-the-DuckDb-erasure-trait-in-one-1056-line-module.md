---
id: TASK-2095
title: 'ARCH-1: data.rs bundles Deadline, Context, registries, schemas, and the DuckDb erasure trait in one 1056-line module'
status: To Do
assignee: []
created_date: '2026-09-07 22:59'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - structure
dependencies: []
parent_task_id: 'TASK-2246'
modified_files:
  - crates/extension/src/data.rs
  - crates/extension/src/lib.rs
priority: low
ordinal: 20000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/data.rs` (whole file, 1056 lines)

**What**: The module doc says "Data provider system: DataProvider trait, DataRegistry, Context, DuckDbHandle" — four-plus concerns behind one 1056-line file: the `Deadline` budget type, the `DataField`/`DataProviderSchema` descriptor types, the `DataProvider` trait, the `DataRegistry`, the `DuckDbHandle` erasure trait (feature-gated), and the per-invocation `Context` with its cache, cycle guard, and deadline machinery.

**Why it matters**: ARCH-1 flags >500-line modules mixing concerns. `Context` + `Deadline` form a cohesive unit (the per-invocation state and its budget) distinct from the registry/schema surface; readers currently navigate a thousand lines of interleaved doc essays to find either. Splitting improves navigability without changing the public API (re-exports in lib.rs already centralize it).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 data.rs split by concern (e.g. context.rs for Context + Deadline, data.rs retaining DataProvider/DataRegistry/schema types); no module exceeds ~500 lines
- [ ] #2 Public API unchanged: lib.rs re-exports the same paths, downstream crates compile without modification
- [ ] #3 cargo test -p ops-extension passes unchanged
<!-- AC:END -->
