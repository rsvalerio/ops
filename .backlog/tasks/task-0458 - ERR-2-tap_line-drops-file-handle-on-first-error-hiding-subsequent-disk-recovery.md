---
id: TASK-0458
title: >-
  ERR-2: tap_line drops file handle on first error, hiding subsequent disk
  recovery
status: Done
assignee:
  - TASK-0537
created_date: '2026-04-28 05:45'
updated_date: '2026-04-28 17:03'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/display.rs:475-485`

**What**: After the first writeln! failure, `self.tap_file = None`; all subsequent step output is silently absent from the tap, even if the underlying condition was transient (NFS hiccup, EAGAIN).

**Why it matters**: For a CI tap file a partial tap is worse than a noisy one — downstream test harnesses may treat an empty tap as "no failures".
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 When tap_file is dropped due to write error, on RunFinished the renderer emits a single warning line (tap file truncated after step X due to: <kind>) to stderr and as the last tap line if at all writable
- [x] #2 Optionally a tap_file_path retry-once strategy is added on first error; if retry succeeds, the handle is kept and a debug message is logged
<!-- AC:END -->
