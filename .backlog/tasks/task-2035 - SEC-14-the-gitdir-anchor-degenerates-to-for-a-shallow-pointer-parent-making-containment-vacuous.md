---
id: TASK-2035
title: >-
  SEC-14: the gitdir anchor degenerates to / for a shallow pointer parent,
  making containment vacuous
status: Triage
assignee: []
created_date: '2026-08-28 23:17'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions/hook-common/src/git.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/git.rs` (`canonical_anchor`)

**What**: The SEC-14 containment anchor is computed as
`parent.ancestors().nth(MAX_GITDIR_PARENT_TRAVERSAL).unwrap_or(parent)` — two
levels above the `.git` pointer file's directory. When the pointer sits near
the filesystem root (`/srv/checkout/.git`, or any two-deep path), that anchor
*is* `/`, and `canonical_target.starts_with("/")` is true for every path on
the machine. The check then costs a syscall and proves nothing.

This is not new — the relative branch has behaved this way since TASK-0788 —
but TASK-1890 extended the same anchor to absolute pointers, so both spellings
now share the degenerate case. The absolute branch has a second, independent
proof (git's `<gitdir>/gitdir` back-reference) that still holds; the relative
branch has only the anchor.

**Why it matters**: `read_gitdir_pointer` feeds `install_hook`, which writes an
executable file git runs on every commit. A containment rule that silently
becomes a no-op depending on how deep the repository happens to live is the
kind of gate that reads as present in review and is absent in the field.
Options: floor the anchor at the pointer's own parent, refuse to resolve when
the computed anchor is the filesystem root, or require the back-reference
proof whenever the anchor degenerates.

**Origin**: discovered during TASK-2008 while fixing TASK-1890.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 canonical_anchor cannot return a path that contains every candidate target (either it is floored below the filesystem root, or resolution is refused with a tracing::debug! breadcrumb when it degenerates)
- [ ] #2 A test plants a pointer whose parent is shallow enough that the anchor would be / and asserts an out-of-tree target is still refused
<!-- AC:END -->
