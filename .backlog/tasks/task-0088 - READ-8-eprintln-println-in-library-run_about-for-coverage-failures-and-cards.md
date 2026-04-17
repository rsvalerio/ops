---
id: TASK-0088
title: >-
  READ-8: eprintln!/println! in library run_about for coverage failures and
  cards
status: To Do
assignee: []
created_date: '2026-04-17 11:33'
updated_date: '2026-04-17 12:07'
labels:
  - rust-codereview
  - read
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/lib.rs:74`

**What**: run_about writes to stderr via eprintln! in library code; also println!s the rendered card.

**Why it matters**: Library code should not write to stdio directly; callers cannot capture, test, or redirect output.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Return a Vec<Warning> (or use tracing::warn!) for the coverage failure branch
- [ ] #2 Accept &mut dyn Write for the card rendering so tests/CLI can supply their own sinks
<!-- AC:END -->
