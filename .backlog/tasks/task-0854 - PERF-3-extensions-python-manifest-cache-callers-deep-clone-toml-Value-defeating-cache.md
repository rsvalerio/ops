---
id: TASK-0854
title: >-
  PERF-3: extensions-python manifest cache callers deep-clone toml::Value,
  defeating cache
status: Triage
assignee: []
created_date: '2026-05-02 09:18'
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
- [ ] #2 Or store the typed projection (e.g., Arc<RawPyproject>) in the cache instead of Arc<toml::Value>, eliminating the second deserialization entirely
- [ ] #3 Add a microbench (or counter test) demonstrating one parse + one project per root
<!-- AC:END -->
