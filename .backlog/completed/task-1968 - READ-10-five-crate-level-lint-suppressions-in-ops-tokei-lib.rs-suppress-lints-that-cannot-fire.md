---
id: TASK-1968
title: >-
  READ-10: five crate-level lint suppressions in ops-tokei lib.rs suppress lints
  that cannot fire
status: Done
assignee:
  - TASK-2012
created_date: '2026-08-27 15:53'
updated_date: '2026-08-28 15:57'
labels:
  - code-review-rust
  - readability
dependencies: []
modified_files:
  - extensions/tokei/src/lib.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/tokei/src/lib.rs:4-12`, `extensions/tokei/src/lib.rs:29`, `extensions/tokei/src/lib.rs:31`

**What**: The crate root carries suppressions for lints that have nothing to suppress in this crate.

1. `#![cfg_attr(test, allow(clippy::unwrap_used, clippy::cast_possible_truncation, clippy::cast_precision_loss, clippy::cast_sign_loss))]` (lines 4-12). Only `unwrap_used` is load-bearing (tests.rs uses `.unwrap()`). The three cast lints fire exclusively on `as` casts, and the crate contains **zero** `as` casts in any of its four files -- `grep -nE '\bas\b' extensions/tokei/src/*.rs` matches only the `use tokei::{Config as TokeiConfig, ...}` import alias and three occurrences of the English word in comments. The workspace additionally denies `clippy::as_conversions`, so a numeric cast could not be added without a separate, deliberate exception anyway.

2. `#[allow(dead_code)]` on `pub const DESCRIPTION` (line 29) and `pub const SHORTNAME` (line 31). Both consts are `pub` items in a library crate root, so `dead_code` never fires on them, and both are in fact consumed by the `ops_extension::impl_extension!` invocation at lines 37-50, which expands them into `fn description()` and `fn shortname()` (crates/extension/src/macros.rs:34-39).

This crate is the template several other extension crates were copied from. The identical dead cast-lint boilerplate was filed for extensions-rust/loc as TASK-1935; the same block also appears in extensions/duckdb, extensions/about, extensions/git and roughly ten more sibling crates, along with the same stale `#[allow(dead_code)]` pair (extensions/duckdb/src/lib.rs:40,42). Fixing the template stops the propagation.

**Why it matters**: READ-10 -- `#[expect(lint, reason = "...")]` would have reported every one of these five suppressions as unfulfilled and they would have been deleted the day they stopped applying. As plain `#[allow]` they are invisible, and they read to the next author as evidence that this crate does arithmetic-heavy cast work and has dead scaffolding, neither of which is true. AGENTS.md states lint levels are centralized in `[workspace.lints]` and an exception must be granted at the narrowest scope that works with the reason next to it; a crate-wide allow for a lint with no callsite is the widest possible scope for no callsite at all.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The crate-root cfg_attr(test, allow(..)) block in extensions/tokei/src/lib.rs lists only clippy::unwrap_used; the three cast lints are removed
- [x] #2 The allow(dead_code) attributes on DESCRIPTION and SHORTNAME are removed and the crate still builds clean under cargo clippy --all-targets --workspace -- -D warnings
- [x] #3 Any suppression that survives is written as expect(lint, reason = ...) with the reason stated inline, per docs/clippy.md
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Fixed in TASK-2012 (branch code-review/TASK-2012).

AC #1 substituted: the block was not reduced to `clippy::unwrap_used`, it was
removed entirely. Writing the survivor as
`#![cfg_attr(test, expect(clippy::unwrap_used, reason = "..."))]` made clippy
report `unfulfilled_lint_expectation` -- proof that `allow-unwrap-in-tests =
true` in `clippy.toml` already relaxes it workspace-wide, so the crate-root
entry suppressed nothing either. AC #1 as written (keep unwrap_used, drop the
three cast lints) and AC #3 (any survivor written as `expect` with a reason)
cannot both hold; removing the block satisfies the intent of both. A comment at
the crate root records why there is no block.

AC #2 and #3 met: the `allow(dead_code)` attributes on DESCRIPTION and SHORTNAME
are gone and `cargo clippy --workspace --all-features --all-targets -- -D
warnings` is clean.

Follow-up filed: TASK-2015 (docs/clippy.md still documents the four-lint block
as universal).
<!-- SECTION:NOTES:END -->
