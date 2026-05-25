---
id: TASK-1388
title: >-
  SEC-25: atomic_write metadata(path) follows symlinks, can broaden destination
  perms via attacker-controlled link target
status: Done
assignee:
  - TASK-1450
created_date: '2026-05-13 18:03'
updated_date: '2026-05-13 19:13'
labels:
  - code-review-rust
  - security
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/edit.rs:192`

**What**: `atomic_write` reads the destination's existing mode with `std::fs::metadata(path)`, which follows symlinks. If `path` is a symlink pointing at an unrelated regular file (e.g. a world-readable 0o644 artifact), the temp file is opened with the *target's* mode and `set_permissions` stamps that mode onto the freshly written file. The subsequent `rename(tmp, path)` then replaces the symlink with a regular file whose perms reflect the unrelated target rather than what a fresh write should default to (`0o600`).

**Why it matters**: SEC-25 hardening (`TASK-1086`) explicitly targeted umask-driven perm narrowing/widening on this codepath, but the symlink-traversal vector remains: an attacker who can place a symlink at `.ops.toml` (or any consumer of `atomic_write`) can broaden the on-disk perms beyond the conservative default. The risk is low in the common single-user workspace, but the SEC-25 contract is "perms never silently widen on atomic-replace" — symlinks defeat that.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Switch the mode probe at edit.rs:192 to symlink_metadata (or equivalent lstat) so the source of perms is the entry itself, not the symlink target
- [ ] #2 When the entry is a symlink (or the metadata probe otherwise yields a non-regular file), fall through to the existing 0o600 default rather than inheriting the symlink's referenced-file mode
- [ ] #3 Add a #[cfg(unix)] regression test that points the destination at a 0o644 sibling via symlink, runs atomic_write, and asserts the resulting regular file at the destination is 0o600 and the prior target is untouched
<!-- AC:END -->
