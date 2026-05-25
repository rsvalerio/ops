---
id: TASK-1503
title: >-
  TEST-2: cargo-toml find_root_strict_rejects_symlinked_ancestor_planting is 110
  lines with weak post-condition
status: To Do
assignee:
  - TASK-1644
created_date: '2026-05-18 18:04'
updated_date: '2026-05-25 16:08'
labels:
  - code-review-rust
  - tests
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/tests/find_root.rs:214-304`

**What**: This single `#[test]` builds two symlink layouts, makes one assertion against `lenient`, then comments out the strict-vs-lenient asymmetry assertion ("Building that race requires tighter symlink surgery than is reliable across CI filesystems"), and ends with a fallback assertion that the strict variant *either* matches lenient *or* returns NotFound. Hits TEST-2 (one concern per test), TEST-12 (mostly setup, weak post-condition), and FN-1 (>50 lines).

**Why it matters**: The test's contract is "pin the surface, not a specific outcome" — that is not a regression detector. A coexisting test (`find_root_strict_skips_off_chain_canonical_ancestor`) already pins the actual contract, so this test's value is unclear.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either tighten the post-condition (assert exactly which variant strict returns on this layout) or remove the test in favour of find_root_strict_skips_off_chain_canonical_ancestor
- [ ] #2 Test body fits FN-1 (<=50 lines) or is split into named-scenario tests
<!-- AC:END -->
