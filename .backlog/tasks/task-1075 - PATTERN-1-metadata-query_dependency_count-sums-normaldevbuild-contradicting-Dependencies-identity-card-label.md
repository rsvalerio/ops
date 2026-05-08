---
id: TASK-1075
title: >-
  PATTERN-1: metadata query_dependency_count sums normal+dev+build,
  contradicting 'Dependencies' identity-card label
status: Done
assignee: []
created_date: '2026-05-07 21:19'
updated_date: '2026-05-08 06:52'
labels:
  - code-review-rust
  - PATTERN
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/views.rs:17` (used by `extensions-rust/about/src/identity/metrics.rs:47`)

**What**: The `crate_dependencies` view exposes `dependency_kind` but the identity-card count sums every row, inflating the displayed number with dev/build deps (often 2-3× normal — serde-test, tempfile, …).

**Why it matters**: Operator-visible misreporting on the about page — what reads as "Dependencies: N" silently includes dev-deps. Either filter to normal-only or rename the card to match what is actually counted.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Decide policy (normal-only vs all-kinds) and document it
- [x] #2 SQL filter (or label) matches the policy
- [x] #3 Regression test with mixed dev/normal deps asserts the rendered count
<!-- AC:END -->
