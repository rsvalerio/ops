---
id: TASK-2018
title: >-
  SEC-38: ops_duckdb::downcast_duckdb calls as_any() on an Arc receiver, so
  get_db and try_provide_from_db never find the DuckDb handle
status: Triage
assignee: []
created_date: '2026-08-28 19:28'
labels:
  - code-review-rust
  - correctness
dependencies: []
modified_files:
  - extensions/duckdb/src/lib.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/lib.rs:35-37` (`downcast_duckdb`), with `try_provide_from_db` (`:57`) and `get_db` (`:67`) as the two callers

**What**: `DuckDbHandle` has a blanket impl covering **every** `'static + Send + Sync` type:

```rust
impl<T: std::any::Any + Send + Sync> DuckDbHandle for T {
    fn as_any(&self) -> &dyn std::any::Any { self }
}
```

`Arc<dyn DuckDbHandle>` is itself `'static + Send + Sync`, so it satisfies that impl too. Method resolution on an `Arc` (or `&Arc`) receiver therefore matches the blanket impl **for the smart pointer** before it ever autoderefs to the inner value, and `as_any()` hands back the erased `Arc` rather than the handle:

```rust
fn downcast_duckdb(handle: Option<&Arc<dyn ops_extension::DuckDbHandle>>) -> Option<&DuckDb> {
    handle.and_then(|h| h.as_any().downcast_ref::<DuckDb>())  // always None
}
```

Verified empirically in `crates/extension/src/tests.rs` while writing TASK-1877's AC#5 coverage: with `handle: Arc<dyn DuckDbHandle>` holding a concrete `FakeDb`, `handle.as_any().downcast_ref::<FakeDb>()` is `None`, `(&handle).as_any().downcast_ref::<FakeDb>()` is `None`, and only `(handle.as_ref() as &dyn DuckDbHandle).as_any().downcast_ref::<FakeDb>()` is `Some`. `handle.as_any().downcast_ref::<Arc<dyn DuckDbHandle>>()` is `Some` — the Arc is what got erased.

**Why it matters**: `get_db` and `try_provide_from_db` are how every DB-backed provider reaches the attached DuckDB connection. Returning `None` unconditionally means `try_provide_from_db` always takes its `fallback_fn` branch and `get_db` always reports no database — silently. There is no error, no warning, and no test that would notice: the fallback recomputes the same data the slower way, so the only symptom is that the DuckDB cache never serves a query it was opened to serve. `DuckDbProvider::provide` still opens the handle and installs it on the context, so the cost is paid and the benefit never collected.

The fix itself is one line (`h.as_ref().as_any()`, or take `&dyn DuckDbHandle` in the signature so the reborrow cannot be forgotten). It is filed rather than applied because it **changes behaviour on live query paths**: `ops about`'s loc / code / coverage / dependency enrichment and the ingest orchestrator would start reading from DuckDB where they currently recompute, and that difference needs its own verification rather than riding a docs-and-tests wave.

Consider also whether the blanket impl should be narrowed (e.g. a sealed marker trait, or `impl DuckDbHandle for DuckDb` only) so an `Arc` receiver stops compiling instead of silently resolving to the wrong impl — the current shape is a design where the wrong call is accepted, which is what TRAIT-9 / TASK-1227 was trying to prevent.

**Origin**: discovered during TASK-1985 (code-review-plan-wave151) while fixing TASK-1877 (AC#5, exercising the duckdb-feature surface). The `ops-extension` side is already closed: `crates/extension/src/data.rs`'s `DuckDbHandle` rustdoc now shows the reborrow, and `as_any_on_an_arc_receiver_erases_the_arc_not_the_handle` pins the resolution behaviour so this task's fix has a clear target.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 downcast_duckdb reborrows as &dyn DuckDbHandle (or its signature takes one) so the downcast reaches the concrete DuckDb
- [ ] #2 A test in ops-duckdb asserts get_db returns Some for a context with an attached DuckDb handle, and try_provide_from_db takes its db_fn branch rather than the fallback
- [ ] #3 Whether the DuckDbHandle blanket impl should be narrowed so an Arc receiver fails to compile is decided and the decision recorded in the trait rustdoc
- [ ] #4 The DB-backed query paths that change behaviour (ops about loc/code/coverage/dependencies, the ingest orchestrator) are verified to still produce the same results now that they actually read from DuckDB
<!-- AC:END -->
