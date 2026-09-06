---
id: TASK-1929
title: >-
  DUP-1: the soft-fail predicate is copied into its own regression test, so the
  test guards a private copy instead of collect_coverage
status: Done
assignee:
  - TASK-2000
created_date: '2026-08-27 15:46'
updated_date: '2026-08-28 15:52'
labels:
  - code-review-rust
  - duplication
dependencies: []
modified_files:
  - extensions-rust/test-coverage/src/parse.rs
  - extensions-rust/test-coverage/src/tests.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/parse.rs:362-368` and `extensions-rust/test-coverage/src/tests.rs:191-215`

**What**: `collect_coverage` decides whether a non-zero `cargo llvm-cov` exit is a soft failure (warn + continue with partial data) or a hard failure (bail with the cargo exit + stderr tail) using an inline closure passed to `Option::filter`:

    raw.get("data").and_then(|d| d.as_array()).is_some_and(|a| {
        !a.is_empty()
            && a.iter()
                .all(|e| e.get("files").and_then(|f| f.as_array()).is_some())
    })

The test that exists to guard that decision, `soft_fail_predicate_rejects_empty_data_array`, does not call `collect_coverage` or any production function. It declares a local closure named `has_valid` whose body is a character-for-character copy of the six lines above, and asserts against the copy. Three assertions (empty `data[]`, populated `data[]`, `data[]` entry with no `files`) all exercise the test's own code.

That makes the test permanently green regardless of what the production predicate does. Invert the `!a.is_empty()`, drop the `.all(...)` clause, or delete the `filter` entirely and the whole suite still passes — while TASK-1557 and TASK-1597 (the two bugs this test was written to pin) both regress silently. The soft-fail branch is the one place in the crate where a real cargo failure can be demoted to a warning, so a regression here converts "cargo could not compile the workspace" into "coverage load completed with zero records".

This is the same rot TASK-1554 already fixed once in this crate: `run_cargo_llvm_cov_arg_list_includes_no_fail_fast` used to grep the source text via `include_str!` and was rewritten to assert against the authoritative `LLVM_COV_ARGS` slice. The predicate never got the same treatment. Sibling precedent outside this crate: TASK-1899 filed the identical shape against `ops-metadata`.

**Why it matters**: a regression guard that tests a copy of the code provides no protection at all, and the reader of the test cannot tell — it looks like a thorough three-case table test. The fix is to give the predicate a name in `parse.rs` (e.g. `pub(crate) fn has_parseable_coverage_data(raw: &serde_json::Value) -> bool`), call it from the `filter` in `collect_coverage`, and have the test call that function. The extraction also removes the only reason the closure had to be duplicated.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The soft-fail predicate exists exactly once, as a named function in parse.rs, and collect_coverage's Option::filter calls it rather than repeating its body
- [x] #2 soft_fail_predicate_rejects_empty_data_array calls that named function; no copy of the predicate body remains anywhere in tests.rs
- [x] #3 A deliberate mutation of the predicate (for example removing the non-empty check) makes soft_fail_predicate_rejects_empty_data_array fail
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Extracted the soft-fail predicate as parse::has_parseable_coverage_data; collect_coverage_with calls it via Option::filter. The guard (tests/collect.rs::soft_fail_predicate_rejects_empty_data_array) now calls that function; the copied closure is gone. Mutation-checked: dropping the !a.is_empty() clause makes the guard fail.
<!-- SECTION:NOTES:END -->
