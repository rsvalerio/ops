---
id: TASK-1859
title: >-
  ERR-7: is_hard_failure stops at the first DbError, so a MutexPoisoned nested
  under DbError::External logs at warn instead of error
status: Done
assignee:
  - TASK-2006
created_date: '2026-08-27 15:29'
updated_date: '2026-08-28 22:04'
labels:
  - code-review-rust
  - error-handling
dependencies: []
modified_files:
  - extensions/duckdb/src/sql/mod.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/mod.rs:361-371` (`is_hard_failure`), consumed by `query_or_warn` at `extensions/duckdb/src/sql/mod.rs:337-355`

**What**: The helper is documented as "walk the anyhow error chain looking for a `DbError` variant that signals trust-broken state (`MutexPoisoned`) or a liveness failure (`Timeout`)", but the loop `return`s on the *first* `DbError` it downcasts, whatever it is:

```rust
while let Some(e) = current {
    if let Some(db) = e.downcast_ref::<DbError>() {
        return matches!(db, DbError::MutexPoisoned(_) | DbError::Timeout { .. });
    }
    current = e.source();
}
```

`DbError::External(#[source] anyhow::Error)` exists precisely to wrap an arbitrary cause graph, and `external_err` is the documented way collectors (`collect_tokei`, `collect_coverage`, `check_metadata_output`) funnel their `anyhow::Error` into `DbError`. `dir.rs`'s own test `external_err_preserves_error_source_chain` proves the wrapped chain survives. So an error shaped `DbError::External(anyhow!(DbError::MutexPoisoned(..)))` — or the same with `Timeout` — downcasts to `External` on the first hop, `matches!` yields `false`, and the walk terminates before ever reaching the hard variant.

The first-match-wins shape is also fragile in the other direction: any future `DbError` variant that legitimately wraps another `DbError` will silently reclassify.

**Why it matters**: The whole point of ERR-7 / TASK-0855 was that a poisoned connection (partially-applied state the connection module explicitly refuses to trust) and a real timeout must not be indistinguishable from a benign `DbError::Io` cache miss in operator logs. For every error that arrives wrapped in `External`, this classifier silently reverts to the pre-fix behaviour: `warn!` in a stream that is already noisy with provider-fallback warnings. The existing tests only cover the un-nested shapes, so the regression is invisible.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 is_hard_failure continues walking the source chain past a non-hard DbError instead of returning on the first downcast
- [x] #2 A test asserts is_hard_failure(DbError::External(anyhow!(DbError::MutexPoisoned(..)).into())) is true
- [x] #3 A test asserts the same for a Timeout nested under External, and that a fully-soft nested chain still returns false
<!-- AC:END -->
