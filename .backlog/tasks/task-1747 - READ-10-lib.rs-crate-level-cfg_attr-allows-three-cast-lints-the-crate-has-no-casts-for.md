---
id: TASK-1747
title: >-
  READ-10: lib.rs crate-level cfg_attr allows three cast lints the crate has no
  casts for
status: Triage
assignee: []
created_date: '2026-08-27 11:14'
labels:
  - code-review-rust
  - structure-readability
dependencies: []
modified_files:
  - extensions-java/about/src/lib.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/lib.rs:14-22`

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

`clippy::unwrap_used` is genuinely needed — the tests use `.unwrap()` throughout. The three `cast_*` allows are dead: the crate contains no numeric cast at all (`grep -n " as \(u\|i\|f\|usize\|isize\)" extensions-java/about/src/` returns nothing; the only arithmetic anywhere is `pom.modules.len()` / `includes.len()`).

Two rules apply:

- **READ-10** — a suppression written as `#[expect(lint, reason = "…")]` deletes itself once the lint stops firing (`unfulfilled_lint_expectations`); written as `#[allow]` with no reason, it outlives the problem it was hiding, which is exactly what happened here. Note the footgun: these are `cfg_attr(test, …)`, so `expect` would need the same cfg scoping.
- **AGENTS.md / docs/clippy.md** — "grant the exception at the narrowest scope that works and write the reason next to it". A crate-level blanket `allow` is the widest possible scope, and it silently covers any cast a future edit adds to the test code.

**Why it matters**: dormant, not a bug — but it is a live blind spot. Any cast added to this crate's test code from now on is pre-approved with no review signal, and the block gives a reader the false impression that this crate does numeric conversion.

**Fix**: drop the three `cast_*` entries. Keep the `unwrap_used` allow (or convert it to `expect` with a reason, mindful of the cfg-scoping footgun) and add a one-line reason next to it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 clippy::cast_possible_truncation, cast_precision_loss and cast_sign_loss are removed from the crate-level cfg_attr allow list
- [ ] #2 The remaining unwrap_used suppression carries a one-line reason, per docs/clippy.md
- [ ] #3 cargo clippy --all-targets --workspace -- -D warnings passes with the allows removed
<!-- AC:END -->
