---
id: TASK-1978
title: >-
  TEST-1: single_ingestion_entry_point has an entirely commented-out body and
  can never fail
status: To Do
assignee:
  - TASK-2012
created_date: '2026-08-27 15:56'
updated_date: '2026-08-28 14:18'
labels:
  - code-review-rust
  - test-quality
dependencies: []
modified_files:
  - extensions/tokei/src/tests.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/tokei/src/tests.rs:373-379`

**What**: The test body is a comment.

    #[test]
    fn single_ingestion_entry_point() {
        // Compile-time guarantee that load_tokei no longer exists; if a future
        // refactor reintroduces it as a public symbol, this test will fail to
        // compile after the corresponding line is uncommented.
        // let _ = super::load_tokei; // intentionally commented out
    }

There is no statement, no assertion, and nothing that can fail. The comment describes a guarantee that is not in force: the only line that would produce the compile error is commented out, so reintroducing `load_tokei` would leave this test passing exactly as it does today. It is a green check mark attached to an unenforced invariant -- worse than no test, because the name in the run output asserts the invariant is covered.

The intent traces to the DUP-1 cleanup noted at lines 326-330 (TASK-0226), which removed `load_tokei` in favour of `TokeiIngestor::load`. That intent is legitimate; the mechanism is not. A negative-existence invariant of this kind is expressible -- a `compile_fail` doctest naming the symbol, or simply deleting the test and relying on the fact that a reintroduced duplicate entry point would be caught in review -- but a commented-out line is not one of the options.

**Why it matters**: TEST-1 -- every test must have meaningful assertions; no empty bodies. Beyond the rule, this one actively misleads: the suite reports coverage for the single-entry-point property while providing none.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 single_ingestion_entry_point either enforces the invariant with a mechanism that actually fails when load_tokei is reintroduced, such as a compile_fail doctest, or is deleted
- [ ] #2 No test in extensions/tokei/src/tests.rs has an empty or fully commented-out body
<!-- AC:END -->
