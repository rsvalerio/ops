---
id: TASK-1943
title: >-
  SEC-25: the fixers rewrite user files with a non-atomic truncating fs::write,
  so an interrupt or ENOSPC leaves the file truncated
status: Done
assignee:
  - TASK-2011
created_date: '2026-08-27 15:48'
updated_date: '2026-08-28 23:36'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions/text-fixers/src/lib.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Severity**: Critical (filed at priority High — the CLI has no `critical` level)

**File**: `extensions/text-fixers/src/lib.rs:144` (`run_fixer`)

**What**: every rewrite is a bare `std::fs::write(&path, &fixed)?`. That is `File::create` (O_WRONLY|O_CREAT|O_TRUNC) followed by `write_all`, with no sync, no temp file, and no rename. Three consequences on a code path whose whole job is rewriting the user's source tree:

1. **Truncation window.** Between the truncate and the completed `write_all` the file on disk is empty or partial. If the process is killed there (Ctrl-C on a pre-commit hook is the normal way users abort one), or the write fails part-way (ENOSPC, EIO, quota), the user's source file is left truncated. There is no backup and no rollback: the only copy of the original was the `bytes` Vec in memory, which dies with the process.

2. **Read/write TOCTOU.** The content is read at line 129 and blindly truncated and overwritten at line 144. Any write to that file in between — an editor autosave, a formatter running as a sibling step of the same `ops verify` run, a rebase — is silently discarded and replaced by the stale in-memory copy plus the whitespace fix. Nothing compares the on-disk bytes before clobbering.

3. **No durability.** Nothing calls `sync_data`/`sync_all`.

**Why it matters**: silent data loss on a tool wired into `.ops.toml` pre-commit (commands = verify, end-of-file-fixer, trailing-whitespace) and into the node/python/terraform/ansible/java stack defaults. `pre-commit-hooks` gets away with the same shape partly because it only touches the *staged* set; here the default walk mode touches the whole worktree (see the companion ERR-6 finding), so the blast radius is every non-ignored file in the repo. A single Ctrl-C during `ops verify` can empty a source file with no trace.

**Suggested fix**: write to a temp file created in the *same directory* (tempfile NamedTempFile new_in parent), write_all + sync_data, then persist / rename over the target. Two things the rename approach must preserve that truncate-in-place gets for free: (a) the target's mode/uid/gid — copy the original Metadata permissions onto the temp file before persisting; (b) hard links and inode identity — a rename breaks hard links and any open fd. Pick one tradeoff deliberately and document it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The rewrite path no longer truncates the target in place: content is written to a temp file in the same directory and moved into place, or an equivalent scheme with no window in which the target is short
- [x] #2 File mode, uid and gid of the original file are preserved across the rewrite, verified by a test that chmods a fixture to 0640 and asserts the mode after a fix
- [x] #3 A test forces the write to fail part-way and asserts the original file content is intact afterwards
- [x] #4 The chosen tradeoff regarding hard links and open file descriptors is stated in a module or function doc comment
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Fixed in TASK-2011. Rewrites now go through the new `extensions/text-fixers/src/atomic.rs`: `tempfile::Builder::tempfile_in` the target's own directory -> `write_all` -> `sync_data` -> copy mode (and uid/gid via `fchown`, EPERM tolerated) -> `persist` (rename(2)) -> parent-dir fsync. There is no window in which the target is short.

AC#4 tradeoff is stated in the `atomic` module doc under "The trade this makes": rename gives the target a new inode, so hard links are broken and open fds keep the old inode; mode/uid/gid are preserved so visible attributes do not change.

Tests: `atomic::tests::preserves_mode` and `tests::file_mode_survives_the_rewrite` (chmod 0640, asserted after the fix) for AC#2; `atomic::tests::a_failed_replace_leaves_the_original_intact` and `tests::a_failing_write_names_the_path_keeps_going_and_leaves_the_file_intact` for AC#3. The permission fixtures self-skip when the process is root (`test_support::ReadOnlyDir` probes rather than assumes).

uid/gid preservation is exercised only in the identity case: handing a file to a different uid needs privilege, so there is no non-root fixture for it.
<!-- SECTION:NOTES:END -->
