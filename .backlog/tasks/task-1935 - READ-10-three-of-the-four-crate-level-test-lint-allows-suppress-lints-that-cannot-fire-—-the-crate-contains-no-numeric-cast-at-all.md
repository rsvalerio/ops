---
id: TASK-1935
title: >-
  READ-10: three of the four crate-level test lint allows suppress lints that
  cannot fire — the crate contains no numeric cast at all
status: Triage
assignee: []
created_date: '2026-08-27 15:46'
labels:
  - code-review-rust
  - readability
dependencies: []
modified_files:
  - extensions-rust/loc/src/lib.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/loc/src/lib.rs:10-18`

**What**: The crate root carries

    #![cfg_attr(
        test,
        allow(
            clippy::unwrap_used,
            clippy::cast_possible_truncation,
            clippy::cast_precision_loss,
            clippy::cast_sign_loss
        )
    )]

Only the first is load-bearing. `src/tests.rs` has 6 `unwrap()` calls, so `clippy::unwrap_used` is genuinely suppressed. The three cast lints suppress nothing: a scan of all four source files for numeric `as` casts finds zero matches, and the two macros expanded into this crate's test code contain none either - `ops_extension::test_datasource_extension!` (crates/extension/src/macros.rs:206) and `ops_duckdb::test_create_sql_validation!` (extensions/duckdb/src/sql/ingest/mod.rs:27) are both assertion-only. The counting code is `u64` end to end via `saturating_add` and never converts between numeric widths.

The block is verbatim boilerplate copied from `extensions/tokei/src/lib.rs:4-12`, where the same three cast allows are equally dead - tokei has no numeric casts either. This is why the copy went unnoticed.

**Why it matters**: A blanket `allow` at crate root is the widest suppression scope there is, and a reader reasonably takes it as a statement that the test code performs lossy numeric conversions. It does not. Worse, the suppression is now permanently pre-authorized: if a future test *does* introduce a truncating cast, the lint that exists to catch it is already switched off crate-wide, silently. READ-10's guidance is that a suppression should not outlive the problem it was hiding; here there was never a problem to hide.

Note the `#[expect]` route is not the fix - READ-10 explicitly warns that `expect` under a cfg-conditional attribute produces `unfulfilled_lint_expectations` warnings in the non-test configuration. The fix is deletion.

Cross-crate cause noted for context only: the same dead allows sit in `extensions/tokei/src/lib.rs`. This finding is filed against `extensions-rust/loc` only, per review scope.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 clippy::cast_possible_truncation, clippy::cast_precision_loss and clippy::cast_sign_loss are removed from the crate-level cfg_attr(test, allow(..)) block
- [ ] #2 clippy::unwrap_used stays, since src/tests.rs relies on it
- [ ] #3 cargo clippy --all-targets -p ops-rust-loc is clean after the removal, confirming the three lints had nothing to suppress
<!-- AC:END -->
