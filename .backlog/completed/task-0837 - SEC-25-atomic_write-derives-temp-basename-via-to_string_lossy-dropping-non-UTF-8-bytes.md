---
id: TASK-0837
title: >-
  SEC-25: atomic_write derives temp basename via to_string_lossy, dropping
  non-UTF-8 bytes
status: Done
assignee: []
created_date: '2026-05-02 09:13'
updated_date: '2026-05-02 12:28'
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
- [x] #1 Use OsStr directly and concatenate via OsString::push rather than going through to_string_lossy
- [x] #2 Add a regression test on Unix using OsString::from_vec(vec![b"a", 0xff]) as the basename
- [x] #3 Document the behaviour explicitly if lossy handling is intended
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
atomic_write now builds the tmp basename via OsString::push from raw OsStr bytes (as_encoded_bytes + from_encoded_bytes_unchecked, ASCII-byte strip is documented-safe). Two non-UTF-8 siblings differing only in invalid bytes now get distinct tmp basenames. Added a Linux-gated regression test (APFS rejects non-UTF-8 names with EILSEQ before the syscall reaches us, hence #[cfg(target_os = "linux")]). Doc comment notes the SEC-25 motivation alongside the existing READ-5 leading-dot stripping.
<!-- SECTION:NOTES:END -->
