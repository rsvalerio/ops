---
id: TASK-1496
title: >-
  PERF-3: run_deps clones the entire serde_json::Value out of the cached Arc
  instead of unwrap_or_clone-ing
status: Done
assignee:
  - TASK-1647
created_date: '2026-05-18 17:29'
updated_date: '2026-05-25 18:53'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/lib.rs:204-205`

**What**:

```rust
let value = ctx.get_or_provide(DATA_PROVIDER_NAME, data_registry)?;
let report: DepsReport = serde_json::from_value((*value).clone())?;
```

`get_or_provide` returns `Arc<serde_json::Value>`. The code dereferences the Arc and deep-clones the entire `Value` tree (which contains the full DepsReport — every upgrade row, every advisory, every license / ban / source entry) just so it can hand the clone to `serde_json::from_value`.

Two cheaper paths:

1. `serde_json::from_value(Arc::unwrap_or_clone(value))` — zero-copy when the Arc has a unique strong count (the common case in `run_deps`, where the value was just produced or fetched once).
2. `DepsReport::deserialize(&*value)` via `serde_json::from_value(value.as_ref().clone())` is no win; the right shape is to call `serde_json::from_value::<DepsReport>` on a borrow with `&Value` → use `DepsReport::deserialize(value.as_ref())` (serde_json's `Value` implements `IntoDeserializer`).

The deep clone is the most expensive shape of all three.

**Why it matters**: PERF-3 — avoid `.clone()` on owned data when a borrow or an `Arc::unwrap_or_clone` would do. For a workspace with hundreds of dependencies the cloned Value is non-trivial (every string field duplicated). Trivial fix, measurable saving.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 run_deps no longer deep-clones the serde_json::Value when destructuring the cached Arc; use Arc::unwrap_or_clone or deserialize from a borrow
- [ ] #2 Behaviour unchanged for cache hits and misses; existing tests pass
<!-- AC:END -->
