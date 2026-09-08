---
id: TASK-2093
title: >-
  PERF-16: CANONICALIZE_CACHE is the crate's one unbounded process-lifetime
  cache (no cap, no eviction)
status: To Do
assignee:
  - TASK-2244
created_date: '2026-09-07 22:59'
updated_date: '2026-09-08 10:58'
labels:
  - code-review-rust
  - performance
dependencies: []
modified_files:
  - crates/core/src/stack/detect.rs
priority: low
ordinal: 19000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/stack/detect.rs:22`

**What**: `static CANONICALIZE_CACHE: OnceLock<Mutex<HashMap<PathBuf, PathBuf>>>` caches canonicalized start paths for the life of the process with no size cap and no eviction. Every sibling cache in this crate carries an explicit documented cap plus an eviction policy and a regression test pinning it:
- `expand.rs` `OPS_ROOT_CACHE_CAP = 64` — LRU via `order: VecDeque` (CONC-1 / TASK-1418), with `ops_root_cache_caps_at_max` asserting the clamp
- `expand.rs` `EXPAND_WARN_SEEN_CAP` — bounded dedup set with its own test
- `text.rs` `BYTE_CAP_ENV_MAX` — the shared env-cap clamp

`CANONICALIZE_CACHE` is the outlier: insert-only, keyed by every distinct `detect()` start path.

**Why it matters**: PERF-16 / SEC-33 — an unbounded HashMap used as a cache grows with the number of distinct keys. Production `ops` dispatch calls `detect()` at most a couple of times per (short-lived) process, so today the exposure is theoretical; but the same crate hosts long-lived test binaries and the doc'd hook callers (`run-before-commit`/`run-before-push` invoke `from_env` repeatedly — PERF-3 / TASK-1465), and the crate's own discipline is that every process-lifetime cache states its bound where the reviewer can see it. The fix is mechanical: mirror the `OPS_ROOT_CACHE_CAP` LRU shape (cap + order queue + eviction on insert) or document at the static why the key cardinality is structurally bounded.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 CANONICALIZE_CACHE gains an explicit cap with oldest-entry eviction mirroring the OPS_ROOT_CACHE_CAP pattern, with a regression test asserting the map clamps at the cap, OR the static carries a comment documenting why distinct-start-path cardinality is structurally bounded in every process that reaches it
- [ ] #2 The chosen bound (or the documented justification) is stated at the CANONICALIZE_CACHE definition so the next cache added to this crate copies the capped shape
<!-- AC:END -->
