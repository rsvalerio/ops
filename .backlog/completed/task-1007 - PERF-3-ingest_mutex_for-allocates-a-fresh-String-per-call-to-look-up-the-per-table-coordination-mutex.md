---
id: TASK-1007
title: >-
  PERF-3: ingest_mutex_for allocates a fresh String per call to look up the
  per-table coordination mutex
status: Done
assignee: []
created_date: '2026-05-04 22:04'
updated_date: '2026-05-05 01:04'
labels:
  - code-review-rust
  - PERF
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/connection.rs:123-129`

**What**: Every call to `ingest_mutex_for(table_name: &str)` allocates `table_name.to_owned()` to feed the `HashMap<String, Arc<Mutex<()>>>::entry(...)`. The branch is *always* taken — `entry` consumes the key whether or not the entry exists, so the alloc happens on every coordination probe, not only on first insert. Each ingest call (every `provide_via_ingestor` invocation, every `--refresh`) then drops the new String immediately because the entry is almost always already present after the first ingest.

**Why it matters**:
- The hot path is `ctx.get_or_provide(provider, registry)` for every `about` subpage / data-provider warmup — 4-6 invocations per `ops about` run today, each landing on `provide_via_ingestor` and then on `ingest_mutex_for`. The per-call allocation is a few hundred bytes (HashMap capacity-doubling notwithstanding) but it's pure overhead: the table_name is one of a handful of `&'static str`s known at compile time.
- The structural fix is `HashMap::raw_entry_mut()` (stable in Rust 1.78+) to do a borrowed lookup that allocates only when inserting; or a `HashMap<&'static str, …>` because every existing call site passes a static. The latter is the boring-Rust answer here — the comment at line 31-37 already constrains map growth to "the database schema", which is a closed set of `&'static str` table names.
- The per-call alloc is also detectable by perf tracing because it dominates the lock-acquire latency for tables that already exist (the no-op case).

**Recommended fix**: change the field to `Mutex<HashMap<&'static str, Arc<Mutex<()>>>>` and have `ingest_mutex_for` take `table_name: &'static str` instead of `&str`. All current call sites already pass static strings (`SidecarIngestorConfig::count_table.as_str()` returns &'static str via TableName::as_str). The signature change is a build error for any caller passing dynamic data — which is exactly the right outcome because the per-table mutex is keyed on the schema.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 ingest_mutex_for no longer allocates a String per call when the entry is already present.
- [ ] #2 Either the field is HashMap<&'static str, …> or the lookup uses raw_entry / get-then-insert to avoid the unconditional to_owned().
- [ ] #3 Existing CONC-7 / TASK-0779 invariant on per-instance scoping is preserved (no leak, drops with DuckDb).
<!-- AC:END -->
