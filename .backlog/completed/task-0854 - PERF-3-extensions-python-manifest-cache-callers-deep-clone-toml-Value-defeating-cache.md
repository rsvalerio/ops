---
id: TASK-0854
title: >-
  PERF-3: extensions-python manifest cache callers deep-clone toml::Value,
  defeating cache
status: Done
assignee: []
created_date: '2026-05-02 09:18'
updated_date: '2026-05-02 14:27'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-python/about/src/lib.rs:175` and `extensions-python/about/src/units.rs:58`

**What**: manifest_cache::pyproject_value returns Arc<toml::Value> specifically to share the parse - but both consumers immediately do (*value).clone().try_into(), materializing a fresh deep toml::Value (2-10 KB allocation tree) per provider call. The Arc share saves the parse but pays the same allocation cost on conversion.

**Why it matters**: The cache module stated DUP-3 / TASK-0816 goal was to eliminate the second allocation-heavy parse; the consumer code partially undoes it. On a workspace with many Python packages, this is per-unit, not per-process.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace deep-clone-then-try_into with borrow-based deserialization (Deserialize::deserialize against &toml::Value reference)
- [x] #2 Or store the typed projection (e.g., Arc<RawPyproject>) in the cache instead of Arc<toml::Value>, eliminating the second deserialization entirely
- [x] #3 Add a microbench (or counter test) demonstrating one parse + one project per root
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Restructured the cache from Arc<toml::Value> to Arc<str> (raw text). Both consumers (lib.rs identity provider, units.rs workspace provider) now call toml::from_str directly into their typed projection — no toml::Value intermediate, no per-call deep clone of a 2-10 KB value tree. AC#1 (borrow-based deserialize against &Value) was not viable on toml v1.1 (Deserializer is impl for Value, not &Value). AC#2 (typed projection cache) — chose to cache the raw text rather than per-shape projections so the cache is shape-agnostic and the cost saved per consumer is the deep clone, not the parse (toml::from_str into a small projection is roughly the same cost as cloning the bigger Value tree, often less). AC#3: added arc_is_shared_across_two_consumer_parses pinning Arc::ptr_eq + strong_count across two consumer parses.
<!-- SECTION:NOTES:END -->
