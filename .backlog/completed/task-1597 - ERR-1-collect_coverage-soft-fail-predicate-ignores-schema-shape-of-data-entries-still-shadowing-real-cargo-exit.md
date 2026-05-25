---
id: TASK-1597
title: >-
  ERR-1: collect_coverage soft-fail predicate ignores schema shape of data[]
  entries, still shadowing real cargo exit
status: Done
assignee:
  - TASK-1634
created_date: '2026-05-21 22:53'
updated_date: '2026-05-22 08:30'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/parse.rs:259-276`

**What**: `has_nonempty_data` only checks that `data` is a non-empty array. If cargo exits non-zero and stdout contains `{"data":[{"unrelated":"junk"}]}` (or any entry missing `files`), the predicate fires and `flatten_coverage_json` errors with "missing or invalid 'files' array in coverage data". The original cargo exit (the real root cause) is shadowed — the same operator-misleading failure TASK-1557 set out to prevent for empty arrays, only partially closed.

**Why it matters**: When cargo really failed, the user sees a parse error pointing at a JSON shape problem instead of "cargo llvm-cov exited with status N: <stderr>". Refines TASK-1557 (Done).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Tighten the soft-fail predicate so every data[] entry must contain a files array (or at least the first entry) before bypassing check_llvm_cov_output
- [ ] #2 When predicate is false on a non-zero exit, surface check_llvm_cov_output(&output)? so the cargo exit + stderr tail wins
- [ ] #3 Regression test feeding status=nonzero stdout=br#"{\"data\":[{}]}"# asserts the error contains 'cargo llvm-cov exited with' rather than 'files'
<!-- AC:END -->
