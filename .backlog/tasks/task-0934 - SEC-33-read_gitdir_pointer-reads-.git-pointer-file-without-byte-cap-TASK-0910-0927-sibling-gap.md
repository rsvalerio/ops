---
id: TASK-0934
title: >-
  SEC-33: read_gitdir_pointer reads .git pointer file without byte cap
  (TASK-0910/0927 sibling gap)
status: Done
assignee: []
created_date: '2026-05-02 15:50'
updated_date: '2026-05-02 16:11'
labels:
  - code-review-rust
  - security
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/git.rs:67`

**What**: `read_gitdir_pointer` calls `std::fs::read_to_string(file)` on the `.git` pointer file with no upper bound. Adversarial repos / multi-GB files / `/dev/zero` symlinks can force unbounded allocation in the `find_git_dir` walk used by every hook installer and provider that resolves a worktree pointer.

**Why it matters**: TASK-0910 added a `MAX_GIT_CONFIG_BYTES` cap to `read_origin_url` and TASK-0927 covers a similar gap on `.git/HEAD`. The `.git` *pointer* file should be subject to the same byte-cap policy — a real worktree pointer is a single line under 4 KiB, but the code path here is reachable by anyone running `ops about` / hook installers in a directory under attacker influence. Symmetric treatment with `MAX_GIT_CONFIG_BYTES` (4 MiB) closes the last unbounded read in the git-dir resolver.

<!-- scan confidence: candidates to inspect -->
- `extensions/hook-common/src/git.rs:67` — `read_to_string` without `File::open + Read::take` cap
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 read_gitdir_pointer caps the read at a documented constant (e.g. MAX_GITDIR_POINTER_BYTES, sibling to MAX_GIT_CONFIG_BYTES)
- [ ] #2 Oversize files are skipped with tracing::warn! and find_git_dir falls through, matching the read_origin_url policy
- [ ] #3 Regression test confirms an oversized .git pointer file does not exhaust memory and yields None
<!-- AC:END -->
