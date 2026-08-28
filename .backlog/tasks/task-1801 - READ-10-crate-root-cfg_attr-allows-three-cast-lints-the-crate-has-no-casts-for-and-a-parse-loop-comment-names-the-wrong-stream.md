---
id: TASK-1801
title: >-
  READ-10: crate-root cfg_attr allows three cast lints the crate has no casts
  for, and a parse-loop comment names the wrong stream
status: To Do
assignee:
  - TASK-1995
created_date: '2026-08-27 11:25'
updated_date: '2026-08-28 14:12'
labels:
  - code-review-rust
  - readability
dependencies: []
modified_files:
  - extensions-rust/cargo-update/src/lib.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-update/src/lib.rs:6-14`, `:141-143`

**What**: Two pieces of stale text in `lib.rs`.

1. The crate-root attribute suppresses four lints in test builds:

```rust
#![cfg_attr(test, allow(
    clippy::unwrap_used,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
))]
```

The crate contains **zero** numeric casts — `grep -cE "\bas +(u|i|f)(8|16|32|64|size)\b" src/lib.rs src/tests.rs` returns `0` for both files. Only `unwrap_used` is
load-bearing (10 `unwrap()` calls in `tests.rs`). The three cast allows are
dead and, being `allow` rather than `expect`, will stay dead silently.
READ-10 is the mechanism: `#[expect(..)]` deletes itself via
`unfulfilled_lint_expectations` once the lint stops firing. (The same pattern
in another crate is filed as TASK-1761 against `extensions-python/about/src/lib.rs`;
this is the `cargo-update` instance.)

2. `:141-143`, inside `parse_update_output`:

```
// At most one increment per line of the in-memory `stdout`
// string, whose length is bounded by `isize::MAX`, so
// `saturating_add` equals `+= 1` exactly.
```

The function takes `stderr: &[u8]` and is called as
`parse_update_output(&output.stderr)` (`:462`) — cargo's dry-run report goes to
stderr, which is the whole reason the provider reads that stream. A comment
naming `stdout` in the one place a reader checks which stream is parsed is
actively misleading, and would make a genuine `stdout`/`stderr` wiring bug look
intentional.

**Why it matters**: dead suppressions blunt the workspace's deny-by-default
lint policy (root `Cargo.toml` `[workspace.lints.clippy]`), which was
deliberately drained of its temporary-allow block by TASK-1671..1682 — the
policy note there says exceptions belong at the call site with a reason. A
crate-root allow for a lint that cannot fire is the opposite. The comment is
a low-cost correctness fix in a file whose comments are otherwise the primary
documentation of eight prior parser bugs.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The three cast-lint allows are removed from the crate-root cfg_attr (or converted to #[expect] so they self-delete), leaving only suppressions that actually fire
- [ ] #2 cargo clippy --all-targets still passes under the workspace deny policy with no extra flags
- [ ] #3 The parse-loop comment at lib.rs:141-143 names stderr, matching the function's parameter and the call site at :462
<!-- AC:END -->
