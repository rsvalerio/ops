---
id: TASK-2399
title: 'SEC-33: read_optional_text can block indefinitely on a FIFO named like a manifest'
status: To Do
assignee: []
created_date: '2026-10-04 14:15'
updated_date: '2026-10-04 14:52'
labels:
  - code-review-rust
  - SEC
dependencies: []
parent_task_id: 'TASK-2421'
modified_files:
  - extensions/about/src/manifest_io.rs
priority: low
ordinal: 1000
dedup_key: 'SEC-33:extensions/about/src/manifest_io.rs:read_optional_text'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/manifest_io.rs:read_optional_text`

**What**: The function opens `path` with `File::open` and reads up to the cap, but never checks that the target is a regular file. In an adversarial repository a FIFO (or other special file) named `package.json`/`pyproject.toml` makes the open block until a writer appears, or the read block on a writer that never closes. The size cap only bounds bytes, not time. The doc comment already considers the `/dev/zero` symlink case but not blocking files.

**Why it matters**: `ops about` runs in user-chosen directories; a hang on a hostile checkout is a cheap DoS (SEC-33). Low severity as the user must run it in that tree.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Non-regular files are rejected without blocking (e.g. open with O_NONBLOCK or check metadata before reading), returning None with a warn
- [ ] #2 Unix test with a FIFO named like a manifest returns promptly
<!-- AC:END -->
