---
id: TASK-1557
title: >-
  ERR-1: collect_coverage soft-fail path accepts empty 'data' array, masking
  real cargo failure as schema bail
status: Done
assignee:
  - TASK-1577
created_date: '2026-05-19 15:42'
updated_date: '2026-05-19 18:05'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/lib.rs:327-346` (`collect_coverage`)

**What**: When `cargo llvm-cov` exits non-zero, the soft-fail branch only checks that `raw.get(\"data\").and_then(|d| d.as_array()).is_some()` before handing off to `flatten_coverage_json`. An empty `data` array satisfies that predicate, but `flatten_coverage_json` then bails with `'data' array is empty in coverage JSON` (line 217). The operator sees a schema-shape error instead of the original cargo exit (which `check_llvm_cov_output` would have surfaced with stderr tail + exit code).

**Why it matters**: ERR-1 — wrong error class. The cargo-failure root cause (compile error, OOM kill, missing toolchain) gets erased and replaced with a misleading 'empty data' message that points at llvm-cov schema drift. Operators chase the wrong lead. Either guard the soft-fail on `!data.is_empty()` (so an empty array falls through to `check_llvm_cov_output(&output)?`) or have `flatten_coverage_json` distinguish empty-with-soft-fail from empty-with-success.

<!-- scan confidence: confirmed by reading source -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Soft-fail predicate in collect_coverage requires a non-empty data array, or flatten_coverage_json empty-array bail propagates the cargo stderr tail + exit code
- [ ] #2 Test pins behaviour: cargo exits non-zero AND stdout contains {"data": []} surfaces the cargo exit code, not the 'data array is empty' message
<!-- AC:END -->
