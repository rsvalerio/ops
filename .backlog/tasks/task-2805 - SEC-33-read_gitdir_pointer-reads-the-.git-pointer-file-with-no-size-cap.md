---
id: TASK-2805
title: 'SEC-33: read_gitdir_pointer reads the .git pointer file with no size cap'
status: To Do
assignee: []
created_date: '2026-11-18 18:46'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - SEC
dependencies: []
parent_task_id: 'TASK-2861'
modified_files:
  - extensions/hook-common/src/git.rs
priority: low
ordinal: 1000
dedup_key: 'SEC-33:extensions/hook-common/src/git.rs:read_gitdir_pointer'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/git.rs:128` (`read_gitdir_pointer`)

**What**: `read_gitdir_pointer` loads the `.git` pointer file with `std::fs::read_to_string(file)`, unbounded. The file is any regular file named `.git` found while walking up from the cwd — the module docs themselves describe it as possibly planted (unpacked archive, vendored tree, scratch directory). The same module already has `read_capped_to_string` (git.rs:356) and applies it, with `MAX_GITDIR_BACKREFERENCE_BYTES`, to the back-reference file — but not to the forward pointer, which is the more exposed of the two reads.

**Why it matters**: A large `.git` file anywhere in the ancestor chain makes every `ops <hook> install` (and any other `find_git_dir` caller) allocate and scan the whole file. Git writes a single `gitdir: <path>` line, so a small cap loses nothing. Using the existing helper also gives the over-cap case the same debug breadcrumb as the back-reference read.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 read_gitdir_pointer reads the pointer through read_capped_to_string (or an equivalent take-bounded read) with a named MAX_* constant
- [ ] #2 An over-cap .git pointer file is refused with a debug breadcrumb, covered by a unit test
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.27 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
