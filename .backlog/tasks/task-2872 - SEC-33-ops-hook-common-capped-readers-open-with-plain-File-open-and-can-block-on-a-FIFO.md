---
id: TASK-2872
title: 'SEC-33: ops-hook-common capped readers open with plain File::open and can block on a FIFO'
status: Triage
assignee: []
created_date: '2026-11-18 22:10'
labels:
  - code-review-rust
  - security
dependencies: []
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
- [ ] #1 Both helpers read through ops_core::text::read_capped_bytes (or document why a hook path must follow symlinks and use an O_NONBLOCK + fstat open instead)
- [ ] #2 Unix tests plant a FIFO at the gitdir back-reference and at an existing hook path and assert the call returns promptly
<!-- AC:END -->
