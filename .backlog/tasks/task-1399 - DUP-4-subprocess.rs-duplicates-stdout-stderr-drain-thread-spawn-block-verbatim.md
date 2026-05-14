---
id: TASK-1399
title: >-
  DUP-4: subprocess.rs duplicates stdout/stderr drain-thread spawn block
  verbatim
status: Done
assignee:
  - TASK-1457
created_date: '2026-05-13 18:09'
updated_date: '2026-05-14 07:58'
labels:
  - code-review-rust
  - DUP
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/subprocess.rs:365-378`

**What**: The two `child.{stdout|stderr}.take().map(|mut s| thread::spawn(move || ... read_capped ...))` blocks differ only in the field name; the rest of the closure body is verbatim copy-paste.

**Why it matters**: Trivial to extract into `fn spawn_drain(pipe, cap) -> Option<JoinHandle<DrainResult>>` so the two halves cannot diverge on the next change to read-cap or panic semantics.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 stdout and stderr drain spawn share a single helper
- [ ] #2 Existing run_with_timeout behavior preserved
<!-- AC:END -->
