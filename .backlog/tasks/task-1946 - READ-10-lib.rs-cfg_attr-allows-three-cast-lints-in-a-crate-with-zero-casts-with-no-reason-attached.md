---
id: TASK-1946
title: >-
  READ-10: lib.rs cfg_attr allows three cast lints in a crate with zero casts,
  with no reason attached
status: Done
assignee:
  - TASK-2000
created_date: '2026-08-27 15:48'
updated_date: '2026-08-28 15:53'
labels:
  - code-review-rust
  - readability
dependencies: []
modified_files:
  - extensions-rust/test-coverage/src/lib.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/lib.rs:16-24`

**What**: the crate root carries

    #![cfg_attr(
        test,
        allow(
            clippy::unwrap_used,
            clippy::cast_possible_truncation,
            clippy::cast_precision_loss,
            clippy::cast_sign_loss
        )
    )]

Only the first suppression does anything. The crate contains no `as` cast at all — not in production code, not in `tests.rs`, not in the inline test modules in `ingestor.rs` and `views.rs` — so `cast_possible_truncation`, `cast_precision_loss`, and `cast_sign_loss` cannot fire under `cfg(test)` or otherwise. They are suppressing nothing and have presumably never suppressed anything in this crate; they look like a template copied across the extension crates.

None of the four carries a `reason`, so a reader cannot tell which are load-bearing. `clippy::unwrap_used` genuinely is — the test module relies on `unwrap`/`expect` throughout, which is the accepted idiom under the ERR-5 scanning guidance — and it should keep an `#[allow]` with a stated reason rather than being folded in with three dead entries.

`#[expect]` is the right tool for the three dead ones (READ-10): it suppresses identically but warns via `unfulfilled_lint_expectations` once the lint stops firing, so a suppression that outlives its cause deletes itself. Here they would warn immediately, which is the correct outcome — they should be deleted. The READ-10 footgun about cfg-dependent expectations does not apply, because these fire in no configuration.

Workspace context: the same three dead cast allows have already been filed against five other crates (TASK-1747, TASK-1761, TASK-1801, TASK-1828, TASK-1883, TASK-1914, TASK-1917), which is what confirms this is a copied template rather than a considered per-crate decision.

**Why it matters**: a lint policy that suppresses lints the code cannot trigger trains readers to skim the block, which is how a real suppression later gets added to it unnoticed. The workspace lint policy lives in the root Cargo.toml [workspace.lints]; per-crate allows should be the rare, explained exception.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The three cast lint allows are removed from the crate-root cfg_attr, and the crate still builds and clippy-cleans under the workspace lint policy
- [x] #2 The remaining clippy::unwrap_used suppression carries a stated reason explaining that test code uses unwrap and expect as its failure mechanism
- [x] #3 No new -W or -A clippy flags are introduced anywhere; the workspace [workspace.lints] policy remains the single source of lint configuration
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
The three dead cast allows are removed; the remaining clippy::unwrap_used allow carries reason = "test code uses unwrap and expect as its failure mechanism (ERR-5 scanning guidance)" plus a comment explaining the deletion. No -W/-A flags added; ops verify clippy gate is clean.
<!-- SECTION:NOTES:END -->
