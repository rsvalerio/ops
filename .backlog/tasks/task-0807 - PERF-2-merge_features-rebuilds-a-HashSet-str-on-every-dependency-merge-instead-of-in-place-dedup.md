---
id: TASK-0807
title: >-
  PERF-2: merge_features rebuilds a HashSet<&str> on every dependency merge
  instead of in-place dedup
status: Done
assignee:
  - TASK-0822
created_date: '2026-05-01 06:02'
updated_date: '2026-05-01 07:01'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/inheritance.rs:229-238`

**What**: For each merge_features call, the function builds let mut seen: HashSet<&str> = base.iter().map(...).collect(); plus merged = base.to_vec(); then iterates additional. With workspace.dependencies resolution called once per dep × member crate, and feature lists typically <10 entries, the HashSet allocation dominates over a linear scan.

**Why it matters**: For small N (the typical case), merged.iter().any(|m| m == f) is cheaper than allocating a HashSet, hashing entries, and dropping it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace HashSet with a linear scan or a vec-dedup pass (e.g. if !merged.iter().any(|m| m == f) { merged.push(f.clone()); })
- [ ] #2 Microbench or simple comparison confirms no regression for feature lists up to 50 entries
- [ ] #3 Output equivalence: same merged feature list (additive, order-preserving)
<!-- AC:END -->
