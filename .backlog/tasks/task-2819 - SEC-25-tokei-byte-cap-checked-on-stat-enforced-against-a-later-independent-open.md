---
id: TASK-2819
title: 'SEC-25: tokei byte cap checked on stat, enforced against a later independent open'
status: To Do
assignee: []
created_date: '2026-11-18 18:48'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - security
dependencies: []
parent_task_id: 'TASK-2855'
modified_files:
  - extensions/tokei/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'SEC-25:extensions/tokei/src/lib.rs:screen_entry'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/tokei/src/lib.rs:414` (`screen_entry`, size check via `entry.metadata().len()`), `extensions/tokei/src/lib.rs:258` (`scan_tokei`, `lang.parse(path, &config)` re-opens the path)

**What**: `ScanLimits::file_bytes` is checked against the walk-time `stat` of each candidate, and the file is opened later by path inside `LanguageType::parse`, which reads the whole file into memory. The file can grow or be swapped between the check and the open, for example by a concurrent writer such as a build or log process in the tree, or by a regular file replaced with a FIFO. A regular-file FIFO swap would block the open until the deadline fires. The open reads whatever is there, with no cap.

**Why it matters**: The `ScanLimits` docs present the byte cap as the guard that keeps a multi-GB file from becoming resident. Under concurrent modification it only advises. The impact is bounded: it needs a concurrent writer in the scanned tree, and the result is memory use or a stall, not a privilege change. That makes it low severity.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The byte cap is enforced on the bytes actually read (e.g. open once, check the handle's metadata / read through a take-limited reader), or the residual race is documented on ScanLimits::file_bytes as an accepted limitation
- [ ] #2 A test or doc comment records the chosen behaviour
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.32 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
