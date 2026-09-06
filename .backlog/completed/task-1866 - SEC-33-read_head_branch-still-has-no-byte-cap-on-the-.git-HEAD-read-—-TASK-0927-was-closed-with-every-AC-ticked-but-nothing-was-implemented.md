---
id: TASK-1866
title: >-
  SEC-33: read_head_branch still has no byte cap on the .git/HEAD read —
  TASK-0927 was closed with every AC ticked but nothing was implemented
status: Done
assignee:
  - TASK-2007
created_date: '2026-08-27 15:30'
updated_date: '2026-08-28 23:27'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions/git/src/config.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/config.rs:120-133` (`read_head_branch`)

**What**: `read_head_branch` calls `std::fs::read_to_string(&head_path)` with no `Read::take` bound, while its sibling `read_origin_url` in the same file caps the read at `MAX_GIT_CONFIG_BYTES` via `File::open` + `take` + an explicit raw-byte cap check (`config.rs:150-205`). A `.git/HEAD` that is multi-gigabyte, or a symlink to `/dev/zero`, forces an unbounded allocation in every `ops about` / `git_info` invocation that walks into that repo.

This is a **regression / false close**: TASK-0927 (`SEC-33: read_head_branch has no byte cap on .git/HEAD read (TASK-0910 sibling gap)`) is filed under `.backlog/completed/` with status Done and all three acceptance criteria ticked, including "#1 read_head_branch reads at most MAX_HEAD_BYTES … oversized HEAD returns None with tracing::warn!" and "#3 the cap value is a pub const". `grep -rn MAX_HEAD_BYTES` across the repository matches only that task file — the constant does not exist in any source file, and `git log -S MAX_HEAD_BYTES -- extensions/git` is empty, so the fix was never written. The Done status is currently hiding an open vulnerability from `backlog search` dedup.

**Why it matters**: same DoS class as TASK-0910 — an unprivileged, user-supplied repository path can OOM the CLI. `ops about` runs interactively and inside memory-constrained CI containers, and the reader is reached automatically for any directory whose ancestors contain a `.git`. A real `.git/HEAD` is ~30 bytes, so the cap is trivial; the only reason it is still missing is that the tracking task was marked complete without the code landing.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 read_head_branch reads at most a pub const MAX_HEAD_BYTES (4 KiB is ample for 'ref: refs/heads/<longname>') using the File::open + Read::take shape already used by read_origin_url
- [x] #2 an oversized HEAD returns None and emits one tracing::warn! mirroring the read_origin_url cap-exceeded event, rather than allocating the file
- [x] #3 the cap is enforced on raw bytes before any decoding, matching the TASK-1620 ordering fix in read_origin_url
- [x] #4 a unit test writes a HEAD payload one byte over the cap and asserts None; existing head_branch_* tests still pass
- [x] #5 TASK-0927 is referenced in the fix so the false-closed record is visibly superseded
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
TASK-0927 (falsely closed) is referenced in the MAX_HEAD_BYTES doc comment and the read_head_branch doc, so the superseded record is visible from the code.
<!-- SECTION:NOTES:END -->
