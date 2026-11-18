---
id: TASK-2797
title: 'SEC-33: ops-git read_capped follows symlinks and blocks on a FIFO .git/config or .git/HEAD'
status: To Do
assignee: []
created_date: '2026-11-18 18:46'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - security
dependencies: []
parent_task_id: 'TASK-2841'
modified_files:
  - extensions/git/src/config.rs
  - crates/core/src/text.rs
priority: medium
ordinal: 1000
dedup_key: 'SEC-33:extensions/git/src/config.rs:read_capped'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/config.rs:103` (`read_capped`, called from `read_origin_url` at :160 and `read_head_branch` at :523)

**What**: `read_capped` opens `<git_dir>/config` and `<git_dir>/HEAD` with a plain `std::fs::File::open`. That call follows a symlink at the final component and, on Unix, blocks indefinitely when the path is a FIFO (open(2) for read on a FIFO waits for a writer). The byte cap (`MAX_GIT_CONFIG_BYTES` / `MAX_HEAD_BYTES`) bounds memory, which is the only threat the doc comment considers ("multi-GB file or a symlink to `/dev/zero`"); it does nothing for an open that never returns. A `.git/HEAD` or `.git/config` planted as a named pipe in a cloned or extracted repository therefore hangs `GitInfo::collect`, so the `git_info` provider and every `project_identity` provider that uses `resolve_repository_with_git_fallback` hang with it (`ops about`). A symlinked `.git/config` also lets a repository point the parser at an arbitrary file outside the checkout.

The workspace already has the hardened primitive: `ops_core::text::open_refusing_symlinks` (O_NOFOLLOW + O_NONBLOCK + fstat S_IFREG check, `crates/core/src/text.rs:192`) behind `read_capped_to_string`. It is `pub(crate)` and the public wrapper hardwires `manifest_max_bytes()` as the cap, so ops-git re-implemented the cap logic (`take(cap+1)` + `try_from` comparison, a near-copy of `read_capped_to_string_with`) without the open hardening. `ops_hook_common::find_git_dir` already refuses a symlinked `.git`, so the gap is only in the files read inside it.

No test covers a FIFO or symlinked `.git/config` / `.git/HEAD`.

**Why it matters**: A hostile checkout can hang ops indefinitely (DoS on untrusted input with no timeout) and can redirect the config read outside the repository. The duplicated cap logic also means a future hardening of the core reader will not reach this crate.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Opening .git/config and .git/HEAD refuses symlinks and non-regular files (FIFO, socket, device) without blocking, e.g. by reusing a public ops_core::text byte-capped reader that takes an explicit cap
- [ ] #2 A refused open is reported as absent with one tracing::warn! naming the file, matching the existing read_capped error handling
- [ ] #3 Unix tests cover a FIFO and a symlink planted at .git/HEAD and .git/config; GitInfo::collect returns promptly with the affected fields None
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.26 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
