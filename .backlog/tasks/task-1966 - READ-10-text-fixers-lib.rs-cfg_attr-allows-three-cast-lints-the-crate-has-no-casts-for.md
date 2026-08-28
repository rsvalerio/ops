---
id: TASK-1966
title: >-
  READ-10: text-fixers lib.rs cfg_attr allows three cast lints the crate has no
  casts for
status: To Do
assignee:
  - TASK-2011
created_date: '2026-08-27 15:53'
updated_date: '2026-08-28 14:18'
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
- [ ] #1 The crate-level cfg_attr no longer allows cast_possible_truncation, cast_precision_loss or cast_sign_loss
- [ ] #2 cargo clippy --all-targets --workspace -- -D warnings is clean after the removal
- [ ] #3 The remaining suppression uses expect with a reason, or the reason is documented next to it per docs/clippy.md
<!-- AC:END -->
