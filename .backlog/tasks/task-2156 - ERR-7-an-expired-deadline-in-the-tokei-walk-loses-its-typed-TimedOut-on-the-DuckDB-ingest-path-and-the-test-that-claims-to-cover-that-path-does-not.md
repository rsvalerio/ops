---
id: TASK-2156
title: >-
  ERR-7: an expired deadline in the tokei walk loses its typed TimedOut on the
  DuckDB ingest path, and the test that claims to cover that path does not
status: To Do
assignee:
  - TASK-2234
created_date: '2026-09-08 07:04'
updated_date: '2026-09-08 10:53'
labels:
  - code-review-rust
  - error-handling
dependencies: []
modified_files:
  - extensions/tokei/src/ingestor.rs
  - extensions/tokei/src/tests.rs
  - extensions/duckdb/src/sql/ingest/orchestrator.rs
priority: medium
ordinal: 69000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/tokei/src/ingestor.rs:18` (`TokeiIngestor::collect`), `extensions/tokei/src/tests.rs:802`

**What**: `scan_tokei` raises an expired budget as `deadline.check()?`, i.e. `anyhow::Error::new(DataProviderError::TimedOut { .. })`. Two paths carry it out:

- **Fallback path** (no `DuckDb` attached): `try_provide_from_db` calls `collect_tokei` and does `.map_err(Into::into)`. `impl From<anyhow::Error> for DataProviderError` (`crates/extension/src/error.rs:292`) downcasts, the top-level object *is* `DataProviderError`, and `TimedOut` survives.
- **Ingest path** (the production shape — a `DuckDb` is attached and `tokei_files` is empty): `TokeiIngestor::collect` does `.map_err(external_err)`, producing `DbError::External(anyhow)`. The orchestrator then wraps it: `ingestor.collect(ctx, &dir).with_context(|| format!("provide_via_ingestor({table_name}): ingestor collect"))?` (`extensions/duckdb/src/sql/ingest/orchestrator.rs:135-137`). Because `collect` returns `Result<_, DbError>` and not `Result<_, anyhow::Error>`, anyhow installs the `context_downcast::<C, DbError>` vtable, which matches only `C` or `DbError` and **does not recurse** (the recursing variant, `context_chain_downcast`, is used only when the inner error is already an `anyhow::Error` — anyhow 1.0.104 `src/error.rs:815-866`). So `From<anyhow::Error> for DataProviderError` fails to downcast and the timeout arrives as `DataProviderError::ComputationFailed`.

The test `a_spent_budget_aborts_the_tokei_walk_with_a_typed_timeout` asserts a typed `TimedOut` and its doc comment claims it "pins the whole path an operator's dispatch takes — not just `scan_tokei`'s parameter". It uses `Context::test_context(..)`, which attaches no database, so it exercises only the fallback branch — the one branch that already works.

**Why it matters**: on a real project the dispatch is the ingest path, so the one failure mode the deadline exists to report distinctly — "this provider ran out of budget" — reaches the operator as a generic computation failure wrapped in an ingest-phase string. Callers that match on `DataProviderError::TimedOut` (retry policy, budget accounting, the "provider was slow" reporting) never see it. The same wrapping applies to every `SidecarIngestorConfig`-based ingestor, so the fix likely belongs in `external_err`/the orchestrator rather than in tokei alone; tokei is where it is observable and tested.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A spent provider budget surfaces as DataProviderError::TimedOut when a DuckDb handle is attached and the ingest path runs, not as ComputationFailed
- [ ] #2 The conversion is fixed at the layer that erases the type (external_err / DbError::External / the orchestrator's with_context), so every sidecar ingestor benefits rather than tokei alone
- [ ] #3 a_spent_budget_aborts_the_tokei_walk_with_a_typed_timeout (or a sibling test) drives the ingest path with an attached DuckDb and asserts the typed TimedOut there
- [ ] #4 The doc comment on that test no longer claims coverage it does not have
<!-- AC:END -->
