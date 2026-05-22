---
id: TASK-1609
title: >-
  READ-5: test-coverage parse.rs collect_coverage uses expect() to recover
  parsed Value already proven non-empty
status: Done
assignee:
  - TASK-1634
created_date: '2026-05-22 06:48'
updated_date: '2026-05-22 08:51'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/parse.rs:273-275`

**What**: After `has_nonempty_data` is computed via `parsed.as_ref().and_then(...).is_some_and(...)`, the code calls `parsed.as_ref().expect("non-empty implies parsed is Some")` to recover the parsed value in the soft-fail branch. The `expect` is provably infallible today (ERR-5 downgrade) but encodes the invariant via a runtime panic message + comment rather than via control flow.

**Why it matters**: A future refactor of `has_nonempty_data` (or its inlined predicate) could silently desynchronise from the `expect` precondition, turning a sound branch into a panic. Restructuring with `if let Some(parsed) = parsed.as_ref().filter(|raw| raw.get("data").and_then(|d| d.as_array()).is_some_and(|a| !a.is_empty()))` keeps the soundness in the type system. READ-5: invariants belong in types, not comments.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Non-zero-exit-with-parseable-JSON branch in collect_coverage reaches flatten_coverage_json without expect()/unwrap()
- [ ] #2 Behavioural contract (warn + flatten when parsed and non-empty data; otherwise fall through to check_llvm_cov_output) unchanged
- [ ] #3 Existing soft-fail tests in tests.rs still pass
<!-- AC:END -->
