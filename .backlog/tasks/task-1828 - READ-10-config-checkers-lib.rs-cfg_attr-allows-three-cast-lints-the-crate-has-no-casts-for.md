---
id: TASK-1828
title: >-
  READ-10: config-checkers lib.rs cfg_attr allows three cast lints the crate has
  no casts for
status: To Do
assignee:
  - TASK-2004
created_date: '2026-08-27 11:34'
updated_date: '2026-08-28 14:15'
labels:
  - code-review-rust
  - structure-readability
dependencies: []
modified_files:
  - extensions/config-checkers/src/lib.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/config-checkers/src/lib.rs:10-18`

**What**:

```rust
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

`clippy::unwrap_used` is load-bearing — the tests use `.unwrap()` throughout. The three `cast_*` allows are dead: the crate contains no numeric cast at all. `grep -n " as \(u\|i\|f\)[0-9]" extensions/config-checkers/src/*.rs` returns nothing across all three files; the only numeric work in the crate is `saturating_add(1)` on the report counters and the `md.len() > opts.max_bytes` comparison, neither of which can fire a cast lint.

This is the same copied preamble flagged in TASK-1747 for `extensions-java/about/src/lib.rs` — a different crate and file, so it is a separate fix, but the two should probably land together since the block is evidently being copy-pasted between new extension crates.

**Why it matters**: READ-10 / rules-classification — a bare `#[allow]` with no rationale and no matching violation is a standing licence for a future edit to introduce a lossy cast in test code with no warning. Suppressions should describe a specific known violation and disappear when it does; `#[expect(..., reason = "...")]` makes that automatic, and here the right answer is simpler still — delete the three that suppress nothing.

**Fix shape**: drop the three `cast_*` entries, keeping `#![cfg_attr(test, allow(clippy::unwrap_used))]`. If the preamble is meant to be a shared house style for extension crates, say so in `docs/clippy.md` rather than duplicating dead allows per crate.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The three cast_* allows are removed from the crate-level cfg_attr in extensions/config-checkers/src/lib.rs
- [ ] #2 clippy::unwrap_used stays suppressed for test builds, and cargo clippy --all-targets --workspace -- -D warnings still passes
<!-- AC:END -->
