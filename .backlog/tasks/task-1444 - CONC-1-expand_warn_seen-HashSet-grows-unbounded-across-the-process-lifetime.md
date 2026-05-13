---
id: TASK-1444
title: 'CONC-1: expand_warn_seen HashSet grows unbounded across the process lifetime'
status: To Do
assignee:
  - TASK-1454
created_date: '2026-05-13 18:44'
updated_date: '2026-05-13 19:09'
labels:
  - code-review-rust
  - concurrency
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/expand.rs:135-151`

**What**: Every distinct `var_name` that ever fails `try_expand` is inserted into the global `HashSet<String>` and never evicted. In a long-running embedder (e.g. `ops` used as a library) a stream of distinct bad variable names — e.g. random `OPS__FOO_<n>` patterns from a misconfigured CI matrix — accumulates per-name allocations indefinitely.

**Why it matters**: Same shape as TASK-1418 (`ops_root_cache` unbounded) but distinct: `expand_warn_seen` is a one-shot dedup hint where drop-on-full is acceptable; `ops_root_cache` holds correctness-critical Arcs. Worth a separate task because the eviction policy differs.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 expand_warn_seen is bounded (e.g. capacity 256) with a documented drop-on-full policy
- [ ] #2 A regression test inserts > capacity distinct var names and asserts the set stays bounded
- [ ] #3 The bound is large enough that realistic command-spec workloads never evict (>= 64)
<!-- AC:END -->
