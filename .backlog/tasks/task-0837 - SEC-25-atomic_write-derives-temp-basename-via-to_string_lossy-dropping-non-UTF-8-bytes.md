---
id: TASK-0837
title: >-
  SEC-25: atomic_write derives temp basename via to_string_lossy, dropping
  non-UTF-8 bytes
status: Triage
assignee: []
created_date: '2026-05-02 09:13'
labels:
  - code-review-rust
  - security
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/edit.rs:82-95`

**What**: When the target path has a non-UTF-8 file name (legal on Unix), to_string_lossy().into_owned() replaces invalid bytes with U+FFFD. The temp file is then created at parent/.<lossy>.tmp.<...>, which is a different basename from the user-supplied target. Two distinct non-UTF-8 file names that differ only in their invalid bytes collide on the same lossy basename.

**Why it matters**: The rename target uses the original path (correct bytes), but the tmp path does not match, so concurrent atomic writes on two siblings whose names differ only in invalid UTF-8 can race on the same tmp.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Use OsStr directly and concatenate via OsString::push rather than going through to_string_lossy
- [ ] #2 Add a regression test on Unix using OsString::from_vec(vec![b"a", 0xff]) as the basename
- [ ] #3 Document the behaviour explicitly if lossy handling is intended
<!-- AC:END -->
