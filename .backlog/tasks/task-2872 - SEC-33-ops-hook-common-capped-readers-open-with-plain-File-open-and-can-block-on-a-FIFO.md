---
id: TASK-2872
title: 'SEC-33: ops-hook-common capped readers open with plain File::open and can block on a FIFO'
status: Done
assignee: []
created_date: '2026-11-18 22:10'
updated_date: '2026-11-19 17:22'
labels:
  - code-review-rust
  - security
dependencies: []
parent_task_id: 'TASK-2881'
modified_files:
  - extensions/hook-common/src/git.rs
  - extensions/hook-common/src/install.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/git.rs:356` (`read_capped_to_string`), `extensions/hook-common/src/install.rs:180` (`read_existing_hook_capped`)

**What**: Both helpers hand-roll the byte cap (`take(cap + 1)` + length comparison) over a plain `std::fs::File::open`, which follows a symlink at the final component and blocks indefinitely in `open(2)` when the path is a FIFO. `read_capped_to_string` in git.rs reads `<gitdir>/gitdir`, where `gitdir` comes from a repo-supplied absolute `.git` pointer target, so a planted named pipe there hangs `find_git_dir`. `ops_core::text::read_capped_bytes` (public since the wave that hardened ops-git's `.git/config` / `.git/HEAD` reads) already provides the hardened open (no-follow component walk, O_NONBLOCK, fstat S_IFREG check) plus the shared cap logic with a typed `CappedReadError`.

**Why it matters**: A hostile checkout can hang hook installation or git-dir discovery (DoS with no timeout), and the duplicated cap logic will not pick up future hardening of the core reader.

**Origin**: discovered during TASK-2841 while fixing TASK-2797.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Both helpers read through ops_core::text::read_capped_bytes (or document why a hook path must follow symlinks and use an O_NONBLOCK + fstat open instead)
- [x] #2 Unix tests plant a FIFO at the gitdir back-reference and at an existing hook path and assert the call returns promptly

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Per call site:
- `<gitdir>/gitdir` back-reference (git.rs has_gitdir_backreference): reads through ops_core::text::read_capped_bytes (strict). The gitdir is canonical, so the prefix holds no symlink, and git writes the back-reference as a plain file.
- `.git` pointer file (git.rs read_gitdir_pointer): reads through the new ops_core::text::read_capped_bytes_following (O_NONBLOCK + fstat S_IFREG on the handle, follows symlinks). The pointer is reached by walking up from an un-canonicalized start dir; a symlinked ancestor (/tmp on macOS, symlinked home) is ordinary and the strict walk would refuse legitimate checkouts. The probe already lstat-checks the final component as a regular file.
- Existing hook (install.rs read_existing_hook_capped): reads through read_capped_bytes (strict). hook_path is under the canonicalized hooks dir and install_hook refuses a symlinked hook first.
The hand-rolled take(cap+1) logic is gone from both helpers. Core gained pub open_regular_following / read_capped_bytes_following; SymlinkPolicy::Follow (global config) now uses the same non-blocking open. Tests: find_git_dir_refuses_fifo_backreference_without_blocking, read_gitdir_pointer_refuses_fifo_without_blocking, install_hook_refuses_fifo_hook_without_blocking, plus core read_capped_bytes_following_refuses_fifo_and_follows_symlinks.
<!-- SECTION:NOTES:END -->
