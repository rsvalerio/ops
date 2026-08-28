---
id: TASK-1915
title: >-
  ARCH-9: the crate re-exports read_stderr_bounded and
  has_staged_files_with_timeout to preserve call sites that no longer exist
status: To Do
assignee:
  - TASK-2009
created_date: '2026-08-27 15:40'
updated_date: '2026-08-28 14:17'
labels:
  - code-review-rust
  - architecture
dependencies: []
modified_files:
  - extensions/run-before-commit/src/lib.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-commit/src/lib.rs:17-22`

**What**:

```rust
// ARCH-1 / TASK-1147: the bounded-wait git-state probe lives in
// `ops_hook_common::git_state` so future hook crates can share it. Re-export
// the public surface here so existing call sites compile unchanged.
pub use ops_hook_common::git_state::{
    has_staged_files_with_timeout, read_stderr_bounded, HasStagedFilesError,
};
```

The stated rationale — "so existing call sites compile unchanged" — no longer describes reality. A workspace grep for each name outside `extensions/hook-common/src/git_state.rs`:

- `read_stderr_bounded` — zero call sites anywhere. It is the internal `recv_timeout` wrapper for the drain channel, called exactly once, from inside `has_staged_files_with_timeout` itself. Nothing outside `git_state.rs` has ever used it.
- `has_staged_files_with_timeout` — used only by this crate's own `#[cfg(test)]` code (lines 95, 297, 331, 369).
- `HasStagedFilesError` — used only by this crate's own `#[cfg(test)]` code (lines 94, 214, 217, 307, 340). It is not the error type of `has_staged_files`, which returns `anyhow::Result<bool>` and erases it.

The only external consumer of this crate is `crates/cli/src/pre_hook_cmd.rs:9-16`, which imports `find_git_dir`, `install_hook`, `ensure_config_command`, `SKIP_ENV_VAR`, `should_skip`, and `has_staged_files` — none of the three re-exported items.

So the crate publishes three symbols solely to serve its own test module, and one of them (`read_stderr_bounded`) is a low-level drain primitive whose own doc in `git_state.rs` describes it as an implementation detail of the bounded wait. `ops_run_before_commit::read_stderr_bounded` invites a caller to treat a private channel helper as part of the pre-commit hook's API.

**Why it matters**: ARCH-9 — keep the public surface minimal and hide internals behind modules. Every re-exported name is a compatibility promise; three that exist only because a test module found them convenient to reach through the crate root make the crate's API look like it offers subprocess plumbing it does not intend to support. Tests can `use ops_hook_common::git_state::{...}` directly (the dependency is already declared), which also documents that they are exercising the shared probe rather than this crate's behaviour.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 read_stderr_bounded is no longer re-exported from ops_run_before_commit
- [ ] #2 has_staged_files_with_timeout and HasStagedFilesError are either dropped from the re-export and imported directly from ops_hook_common::git_state by the tests that need them, or the re-export comment is rewritten to state the actual reason they are public
- [ ] #3 The stale 'so existing call sites compile unchanged' rationale is removed or corrected
<!-- AC:END -->
