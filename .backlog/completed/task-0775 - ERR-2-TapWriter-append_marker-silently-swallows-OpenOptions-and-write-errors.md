---
id: TASK-0775
title: 'ERR-2: TapWriter::append_marker silently swallows OpenOptions and write errors'
status: Done
assignee:
  - TASK-0824
created_date: '2026-05-01 05:56'
updated_date: '2026-05-01 09:54'
labels:
  - code-review-rust
  - errors
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/display/tap.rs:78-84`

**What**: The append-marker path opens with OpenOptions::new().append(true).open(path) and discards the Result on both open and writeln!. If the tap file is unreachable (deleted, perms changed) the user-visible stderr warning from report_tap_truncation still fires, but the marker silently fails.

**Why it matters**: report_tap_truncation already logs a warning on the truncation cause, but a second silent failure leaves a partial tap file with no marker, contradicting the comment that "downstream parser that only inspects the file (no stderr capture) still sees the truncation".
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Log at tracing::warn!(target: "ops::tap") if the append-marker open or writeln fails, distinguishing the two failure modes
- [x] #2 Keep the function infallible (no Result propagation; it is best-effort by design)
- [ ] #3 Optionally test by chmod'ing the tap file read-only after writes start
<!-- AC:END -->
