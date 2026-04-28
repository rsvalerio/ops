---
id: TASK-0451
title: >-
  CONC-5: run_with_timeout polls every 100ms via thread::sleep with no
  cancellation
status: To Do
assignee:
  - TASK-0537
created_date: '2026-04-28 05:44'
updated_date: '2026-04-28 07:14'
labels:
  - code-review-rust
  - CONC
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/subprocess.rs:155-214`

**What**: The bounded-wait helper polls `try_wait` every 100ms for the entire duration of every subprocess. A 30s `cargo metadata` causes 300 wakeups; the 1-hour ceiling causes 36000. There is no signal/cancellation: Ctrl-C kills the parent and leaves the child running until the deadline.

**Why it matters**: Burns CPU/battery on idle waits and defeats macOS App Nap. More importantly, no cancellation means user interrupts during long `cargo deny` runs cannot stop the child cleanly.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace polling loop with wait_timeout from the wait-timeout crate (single OS-level wait), or document why polling cadence was chosen
- [ ] #2 Document/handle SIGINT during run_with_timeout so the child dies with the parent
<!-- AC:END -->
