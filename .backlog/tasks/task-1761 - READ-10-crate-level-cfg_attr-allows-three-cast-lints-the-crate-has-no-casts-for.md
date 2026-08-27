---
id: TASK-1761
title: >-
  READ-10: crate-level cfg_attr allows three cast lints the crate has no casts
  for
status: Triage
assignee: []
created_date: '2026-08-27 11:19'
labels:
  - code-review-rust
  - readability
dependencies: []
modified_files:
  - extensions-python/about/src/lib.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-python/about/src/lib.rs:12-20`

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

`clippy::unwrap_used` is load-bearing — the test modules use `.unwrap()` throughout. The three `cast_*` allows are not: there is no `as` cast, and no numeric conversion of any kind, anywhere in `lib.rs` or `units.rs`. The only numeric field the crate touches is `module_count`, which it sets to `None` (`lib.rs:104`). Verified by grepping both files for `as ` casts — none.

**Why it matters**: `docs/clippy.md` and the root `AGENTS.md` both require lint exceptions to be granted "at the narrowest scope that works" with a reason recorded next to them. These three are crate-wide, unexplained, and cover a lint class the crate cannot trigger — so they are pre-authorisation for casts nobody has reviewed. If a future change introduces a lossy `usize`→`i64` conversion for a package count, clippy will stay silent about it in test builds.

**Note**: the identical block was filed against the Node about crate this run as TASK-1747, so the two were copied from a common template; the sweep should cover every `extensions-*/about/src/lib.rs`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 clippy::cast_possible_truncation, cast_precision_loss and cast_sign_loss are removed from the crate-level cfg_attr
- [ ] #2 The remaining clippy::unwrap_used allow keeps a short comment stating why it is needed for test modules
- [ ] #3 cargo clippy --all-targets --workspace -- -D warnings passes
<!-- AC:END -->
