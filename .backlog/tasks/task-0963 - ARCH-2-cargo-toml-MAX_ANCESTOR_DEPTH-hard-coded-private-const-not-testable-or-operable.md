---
id: TASK-0963
title: >-
  ARCH-2: cargo-toml MAX_ANCESTOR_DEPTH hard-coded private const, not testable
  or operable
status: Triage
assignee: []
created_date: '2026-05-04 21:47'
labels:
  - code-review-rust
  - architecture
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/lib.rs:295-371`

**What**: `MAX_ANCESTOR_DEPTH = 64` is a private const used inside `find_workspace_root`. Tests cannot verify the symlink-loop bound without crafting actual 64-deep dir hierarchies. The same value is reported in the `NotFound` error so users see "walked up to 64 ancestors" with no override path if their layout legitimately needs deeper.

**Why it matters**: Compromises both testability and operability. The "best-effort symlink-safe" guarantee is documented but not configurable; a test for the bound currently does not exist for this reason.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Accept depth as a parameter (with a public default const). High-level entry point keeps the const but a find_workspace_root_with_depth permits test injection
- [ ] #2 Test verifies the depth bound is honored without creating an actual 64-level dir tree
<!-- AC:END -->
