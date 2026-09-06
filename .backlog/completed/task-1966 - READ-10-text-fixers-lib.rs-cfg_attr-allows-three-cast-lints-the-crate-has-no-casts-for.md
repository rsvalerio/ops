---
id: TASK-1966
title: >-
  READ-10: text-fixers lib.rs cfg_attr allows three cast lints the crate has no
  casts for
status: Done
assignee:
  - TASK-2011
created_date: '2026-08-27 15:53'
updated_date: '2026-08-28 23:39'
labels:
  - code-review-rust
  - readability
dependencies: []
modified_files:
  - extensions/text-fixers/src/lib.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Severity**: Low

**File**: `extensions/text-fixers/src/lib.rs:9-17`

**What**:

```
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss
    )
)]
```

Only the first exception is used. A grep for casts across the whole crate — `src/lib.rs`, `src/discovery.rs`, `src/binary.rs`, `src/trailing.rs`, `src/eof.rs` — returns nothing: there is not a single `as` conversion in production or test code. The three cast allows are dead, and being crate-level `allow` (not `expect`) they produce no warning to tell anyone that.

`clippy::unwrap_used` is genuinely needed: the test modules contain 58 `unwrap()` calls.

This is the same copy-paste template flagged on three sibling crates already — TASK-1914, TASK-1828 (`extensions/config-checkers/src/lib.rs`) and TASK-1747 — so fixing it here should follow whatever wording those settle on.

**Why it matters**: `docs/clippy.md` and the AGENTS.md lint policy require exceptions to be granted at the narrowest scope that works, with a reason next to them. Three unused crate-level allows are the widest possible scope for a violation that does not exist, and they will silently absorb a real cast introduced later — in a crate whose arithmetic is all buffer offsets, a `cast_possible_truncation` that goes unwarned is not harmless.

**Suggested fix**: drop the three cast allows, keeping only `clippy::unwrap_used`, and prefer `#[expect(...)]` over `#[allow(...)]` (READ-10) so the suppression fails loudly if it stops being needed.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The crate-level cfg_attr no longer allows cast_possible_truncation, cast_precision_loss or cast_sign_loss
- [x] #2 cargo clippy --all-targets --workspace -- -D warnings is clean after the removal
- [x] #3 The remaining suppression uses expect with a reason, or the reason is documented next to it per docs/clippy.md
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Fixed in TASK-2011. The whole `#![cfg_attr(test, allow(..))]` block is removed, not just the three cast lints (AC#1).

`clippy::unwrap_used` turned out to be dead here too: `clippy.toml` sets `allow-unwrap-in-tests = true` workspace-wide, so the lint never fires in a `#[cfg(test)]` module in the first place. Writing it as `#[expect]` proves it — clippy reports the expectation as unfulfilled, which is how this was caught. This is the same conclusion TASK-1968 reached for `extensions/tokei`, so the crate root now carries the same explanatory comment in place of the block, naming this task and both reasons: no `as` cast exists anywhere in the crate (and the workspace denies `clippy::as_conversions` anyway), and `unwrap_used` is already relaxed by `clippy.toml`. That satisfies AC#3 — `expect` is not usable, so the reason is documented next to the (absent) suppression per `docs/clippy.md`.

AC#2: `cargo clippy --workspace --all-targets -- -D warnings` is clean.

The finding's worry was concrete for this crate: the arithmetic here is all buffer offsets, and the three cast allows would have absorbed the first `cast_possible_truncation` anyone introduced. The wave added `usize::try_from` / `u64::try_from` conversions in `runner::read_bounded`, and with the allows gone clippy is what keeps them honest.
<!-- SECTION:NOTES:END -->
