---
id: TASK-1883
title: >-
  READ-10: lib.rs carries four lint suppressions that suppress nothing — three
  cast allows with no casts in the crate, and module_inception on a module that
  is not nested
status: Triage
assignee: []
created_date: '2026-08-27 15:33'
labels:
  - code-review-rust
  - readability
dependencies: []
modified_files:
  - crates/extension/src/lib.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/lib.rs:3-17`

**What**: the crate root opens with two suppression blocks, and every entry in them is inert:

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
...
#[allow(clippy::module_inception)]
mod macros;
```

- `clippy::unwrap_used` in tests is already granted workspace-wide by `allow-unwrap-in-tests = true` in `/home/rsvalerio/Projects/ops/clippy.toml`, alongside the matching keys for `expect`, `panic` and indexing. The crate-root `cfg_attr` duplicates that policy in a second place — exactly the lint-configuration duplication ARCH-11 asks workspaces to avoid.
- The three cast lints have nothing to fire on: `crates/extension/src/tests.rs` contains no `as` cast at all (862 lines, zero numeric casts), and neither does any other file in the crate.
- `clippy::module_inception` fires on a module nested inside a module of the same name (`mod macros { mod macros { … } }`). `macros.rs` declares no inner module — it contains three `macro_rules!` definitions and nothing else — so the lint could not fire here either.

Verified: `cargo clippy -p ops-extension --all-targets --all-features` is clean, and it stays clean with these removed since none of them is suppressing anything.

**Why it matters**: READ-10's point is that a suppression should delete itself once the problem it hid is gone. These four outlived their causes and are now misinformation: a reader encountering `#[allow(clippy::module_inception)]` reasonably concludes `macros.rs` has a nesting problem worth working around, and the cast allows imply the crate does numeric conversion it does not do. They also form a standing permission — if someone later adds a truncating cast to this crate's tests, the lint that the workspace denies at pedantic level will silently not fire.

**Suggested fix**: delete `#[allow(clippy::module_inception)]` and the three cast entries. Drop `clippy::unwrap_used` too, since `clippy.toml` already covers it — if the `cfg_attr` block ends up empty, remove it entirely. Where a suppression is genuinely needed in future, prefer `#[expect(lint, reason = "…")]` per READ-10 so it warns via `unfulfilled_lint_expectations` once it stops being needed, rather than sitting inert for another year.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The three cast allows and the module_inception allow are removed from crates/extension/src/lib.rs
- [ ] #2 clippy::unwrap_used is removed from the crate-root cfg_attr block in favour of the workspace-level allow-unwrap-in-tests setting in clippy.toml, and the block is deleted if it ends up empty
- [ ] #3 cargo clippy -p ops-extension --all-targets --all-features remains clean with no warnings after the removals
<!-- AC:END -->
