---
id: TASK-1914
title: >-
  READ-10: crate-level cfg_attr allows three cast lints the crate has no casts
  for
status: Done
assignee:
  - TASK-2010
created_date: '2026-08-27 15:40'
updated_date: '2026-08-28 15:13'
labels:
  - code-review-rust
  - readability
dependencies: []
modified_files:
  - extensions/run-before-push/src/lib.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-push/src/lib.rs:3-11`

**What**:

    #![cfg_attr(
        test,
        allow(
            clippy::unwrap_used,
            clippy::cast_possible_truncation,
            clippy::cast_precision_loss,
            clippy::cast_sign_loss
        )
    )]

`clippy::unwrap_used` is load-bearing — the test module uses `.unwrap()` at lines 81, 82, 86, 92, 95. The three cast lints suppress nothing: the crate contains no numeric cast at all (`grep -n ' as [a-z0-9]' extensions/run-before-push/src/lib.rs` is empty; the file is 119 lines and has no arithmetic outside string assertions). The block is a verbatim copy of the header in `extensions/run-before-commit/src/lib.rs:3-11`, where the cast allows plausibly did have a subject at some point.

`#[allow]` is silent when it suppresses nothing, so these three outlive whatever justified them and read to the next author as "this crate does numeric casting in tests" — which it does not.

**Why it matters**: READ-10 — a suppression that is meant to cover a specific known violation should be `#[expect(..., reason = "...")]`, which warns via `unfulfilled_lint_expectations` once the violation is gone and so deletes itself. Here the right outcome is simply to drop the three cast entries and keep `unwrap_used`, which is a permanent, intentional policy exception for the test cfg and can stay an `#[allow]`. Same pattern already filed for other crates (TASK-1747, TASK-1761, TASK-1801, TASK-1828, TASK-1883); this is the `run-before-push` instance, which none of those cover.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The three cast lint entries (cast_possible_truncation, cast_precision_loss, cast_sign_loss) are removed from the crate-level cfg_attr
- [x] #2 clippy::unwrap_used remains suppressed under cfg(test) and carries a short reason next to it
- [x] #3 cargo clippy --all-targets --workspace -- -D warnings passes after the change
<!-- AC:END -->
