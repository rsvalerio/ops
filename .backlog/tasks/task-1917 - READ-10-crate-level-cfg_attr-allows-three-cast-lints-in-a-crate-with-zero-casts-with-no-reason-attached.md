---
id: TASK-1917
title: >-
  READ-10: crate-level cfg_attr allows three cast lints in a crate with zero
  casts, with no reason attached
status: To Do
assignee:
  - TASK-2009
created_date: '2026-08-27 15:41'
updated_date: '2026-08-28 14:17'
labels:
  - code-review-rust
  - readability
dependencies: []
modified_files:
  - extensions/run-before-commit/src/lib.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-commit/src/lib.rs:3-11`

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

The crate contains no `as` cast at all — `grep -nE " as (u|i|f)[0-9]" extensions/run-before-commit/src/lib.rs` returns nothing, and there is no arithmetic on numeric types anywhere outside `Duration::from_secs`. Three of the four allows suppress lints that cannot fire. Only `clippy::unwrap_used` is load-bearing (the test module uses `.unwrap()` throughout).

None of the four carries a `reason`, and all four are granted at crate root — the widest possible scope — which contradicts AGENTS.md ("grant the exception at the narrowest scope that works and write the reason next to it") and `docs/clippy.md`. The block is copy-pasted verbatim into `extensions/run-before-push/src/lib.rs:3-11` and `extensions/hook-common/src/lib.rs:12-20`, so it reads as boilerplate nobody has re-derived; the same pattern has already been filed for other crates as TASK-1747 (extensions-java/about) and TASK-1828 (config-checkers), which is why it is worth fixing at the source rather than crate by crate forever.

**Why it matters**: READ-10 — `#[expect(lint, reason = "…")]` suppresses identically but warns via `unfulfilled_lint_expectations` once the lint stops firing, so the suppression deletes itself instead of outliving the problem. Here it would have fired on day one for all three cast lints and the block would never have been copied to two more crates. A reason-free blanket allow at crate root also earns no severity reduction under the classification guidance, and it silently pre-authorises any future cast the crate gains: someone adding `elapsed.as_secs_f64() as u32` to a test gets no warning, in a crate whose whole job is timing a subprocess.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The three cast-lint allows are removed from the crate-level cfg_attr (cargo clippy --all-targets --workspace -- -D warnings still passes)
- [ ] #2 The remaining unwrap_used suppression uses #[expect(..., reason = "...")] or carries an adjacent comment stating why test code is exempt
- [ ] #3 The suppression is scoped to the #[cfg(test)] module rather than the crate root, if that scope compiles clean
<!-- AC:END -->
