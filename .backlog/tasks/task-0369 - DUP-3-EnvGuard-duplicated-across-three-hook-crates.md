---
id: TASK-0369
title: 'DUP-3: EnvGuard duplicated across three hook crates'
status: To Do
assignee:
  - TASK-0421
created_date: '2026-04-26 09:37'
updated_date: '2026-04-26 10:10'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-push/src/lib.rs:66` (also run-before-commit and hook-common)

**What**: Identical EnvGuard struct + Drop impl appears in run-before-push, run-before-commit, and hook-common test modules — three near-byte-identical copies.

**Why it matters**: Maintenance burden and divergence risk; any future fix (Rust 2024 unsafe, fallibility, multiple-var support) must be replicated in three places.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Move EnvGuard into hook-common behind a test-helpers feature and reuse from the wrapper crates
- [ ] #2 Three duplicated definitions deleted; tests still pass under cargo test --all-features
<!-- AC:END -->
