---
id: TASK-1894
title: >-
  READ-4: has_staged_files_with_timeout documents a # Panics contract for a
  panic the function deliberately no longer has
status: To Do
assignee:
  - TASK-2008
created_date: '2026-08-27 15:35'
updated_date: '2026-08-28 14:17'
labels:
  - code-review-rust
  - readability
dependencies: []
modified_files:
  - extensions/hook-common/src/git_state.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/git_state.rs:128-131`

**What**: The public doc carries:

```
/// # Panics
///
/// If the spawned child has no stderr pipe, which cannot happen because the
/// command is configured with `Stdio::piped()`.
```

The body does not panic. The `None` arm was deliberately rewritten to drop the sender instead, and says so in its own comment (`git_state.rs:172-175`):

```rust
// `.stderr(Stdio::piped())` above guarantees `Some`. Dropping the
// sender in the impossible arm keeps `read_stderr_bounded` from
// waiting out its full grace period instead of panicking.
None => drop(stderr_tx),
```

There is no `unwrap`, `expect`, `panic!`, `unreachable!`, or slice index anywhere in the function, so the section is a leftover from the pre-fix shape.

**Why it matters**: `# Panics` is a contract, not commentary. Callers on a git-hook critical path read it to decide whether they need `catch_unwind` or a supervising wrapper, and the crate's stated policy is that hooks fail with typed errors rather than panics (`HasStagedFilesError` exists for exactly that). Advertising a panic that cannot occur pushes callers toward defensive scaffolding they do not need, and — worse for maintenance — invites a future edit to "restore" the `unwrap` the section describes, undoing the drop-the-sender fix and its documented grace-period benefit. Deleting the section (or restating it as the invariant note it actually is) is the whole change.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The # Panics section on has_staged_files_with_timeout is removed, since the function has no panicking path
- [ ] #2 The Stdio::piped() invariant it described is preserved as a plain note (the inline comment on the None arm already states it), so the reasoning is not lost
<!-- AC:END -->
