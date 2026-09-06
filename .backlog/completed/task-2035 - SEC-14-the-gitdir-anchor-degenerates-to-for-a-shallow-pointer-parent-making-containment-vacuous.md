---
id: TASK-2035
title: >-
  SEC-14: the gitdir anchor degenerates to / for a shallow pointer parent,
  making containment vacuous
status: Done
assignee:
  - TASK-2041
created_date: '2026-08-28 23:17'
updated_date: '2026-08-29 12:55'
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
- [x] #1 canonical_anchor cannot return a path that contains every candidate target (either it is floored below the filesystem root, or resolution is refused with a tracing::debug! breadcrumb when it degenerates)
- [x] #2 A test plants a pointer whose parent is shallow enough that the anchor would be / and asserts an out-of-tree target is still refused
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Verified on code-review/run-20260828-part3 during the PR #41 CodeRabbit follow-up: the report is accurate and the finding still stands.

canonical_anchor() is parent.ancestors().nth(2).unwrap_or(parent) with no floor, so for any pointer whose parent is two or fewer components deep the anchor canonicalizes to / and canonical_target.starts_with("/") is true for every path on the machine. Both the relative branch (read_gitdir_pointer) and the absolute branch (resolve_absolute_gitdir) share it, so in that layout the SEC-14 containment gate costs a canonicalize syscall and proves nothing.

Context added by this run: resolve_absolute_gitdir gained a third acceptance rule (commit aa0aeacb) for the --separate-git-dir layout, which git writes with no reverse link in either direction. That rule is a substance check on the target (HEAD + objects/ + refs/), not a containment proof, so it does not depend on the anchor and does not make this task worse — but it does mean the absolute branch now has two of three rules that are not containment. Fixing the degenerate anchor is still worth doing for the relative branch, which has the anchor and nothing else.

Existing tests do not catch it because every fixture plants its pointer deep inside a tempdir (>2 components), where the anchor lands inside the fixture and does discriminate. AC#2's shallow-parent test is the missing coverage.

Fixed by flooring the anchor: canonical_anchor now picks its ancestor through the new anchor_ancestor(), which climbs at most MAX_GITDIR_PARENT_TRAVERSAL levels but never past the deepest non-root ancestor (/srv/checkout anchors at /srv, not /). When only the root is available, resolution is refused with a tracing::debug! breadcrumb, and a canonical anchor that still resolves to the root is refused the same way. Tests: anchor_ancestor_is_floored_below_the_filesystem_root pins the lexical floor for the shallow layouts a test cannot create on disk; find_git_dir_rejects_out_of_tree_redirect_from_a_shallow_pointer plants the pointer directly at the tempdir root (parent /tmp/.tmpXXXX, exactly the two-deep case) and asserts a symlink jumping out of the temp tree is refused - it was accepted before the floor.
<!-- SECTION:NOTES:END -->
