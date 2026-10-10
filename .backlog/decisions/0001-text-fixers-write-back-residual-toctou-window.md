# ADR 0001: Accept the residual TOCTOU window in `ops-text-fixers`' write-back

- **Date**: 2026-10-10
- **Status**: Accepted (TASK-2434, SEC-25)
- **Scope**: `extensions/text-fixers/src/atomic.rs`

## Context

`ops-text-fixers` rewrites source files in place: the fixed content is staged
in a sibling temp file and `rename(2)`d over the target. The read side is
handle-based and race-free — the file is opened through
`ops_core::text::open_refusing_symlinks`, which walks every path component
with `openat(2)` under `O_NOFOLLOW` and reads through the resulting
descriptor. The write side, however, only has the path: it re-`lstat`s the
target (device, inode, length, mtime) right before the rename and refuses a
symlinked directory component, but both the checks and the stage/rename
syscalls themselves resolve the path by name. A directory swapped in between
a check and the syscall it guards, or an edit landing in the one-syscall gap
between the `lstat` and the `rename`, is not seen.

Closing the gap structurally means running the whole write-back relative to a
parent-directory handle: `openat` the parent with `O_NOFOLLOW` per component,
stage with `openat(O_CREAT | O_EXCL | O_NOFOLLOW)`, `fstatat` the target, and
`renameat` — the shape `ops-sqlite`'s `IngestDir::write_atomic` already uses.
That is FFI, and `ops-text-fixers` holds `unsafe_code = "deny"`; the natural
home would be `ops_core::text` next to `open_refusing_symlinks`, which makes
the change a cross-crate API addition rather than a fix local to this crate.

The exposure is also narrow. The tool runs over the user's own worktree, not
over adversarial input; the window is one syscall wide; and the pre-rename
identity check means an attacker who wins the race must do it inside that
window while holding write access to the worktree — at which point the tree
is already theirs.

## Decision

The residual path-based window is **accepted as a documented limit** rather
than closed with new `*at` FFI in this wave. Two mitigations bound it:

1. The changed-since-read comparison now also compares **`ctime`** on Unix
   (with nanoseconds). `ctime` is kernel-maintained and cannot be restored
   from userspace, so the previously undetected case — a same-length in-place
   edit that also restores the original `mtime` — is now refused.
2. The directory check and the identity check stay directly in front of the
   syscalls they guard, keeping the window at one syscall gap rather than the
   whole read-fix-write cycle.

This ADR is the acceptance record required by TASK-2434's first acceptance
criterion.

## Consequences

- The write-back can still be redirected or racing in the one-syscall window
  between `ensure_unchanged` and `rename(2)`; behaviour in that window is
  undefined by this design and the module docs state it.
- If `ops_core::text` ever grows a handle-based write-back next to
  `open_refusing_symlinks` (the `openat`/`fstatat`/`renameat` shape above),
  `atomic::replace` should adopt it and retire this acceptance.
- Revisit this decision if `ops-text-fixers` ever runs outside a worktree the
  user owns (e.g. over untrusted checkouts in CI), where the "attacker with
  write access already owns the tree" argument no longer holds.
