---
id: TASK-0953
title: >-
  TEST-15: cli_run_command_with_timeout uses real 3s sleep with 1s timeout
  (flake risk)
status: To Do
assignee:
  - TASK-1009
created_date: '2026-05-04 21:45'
updated_date: '2026-05-06 06:47'
labels:
  - code-review-rust
  - test-quality
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `tests/integration.rs:253-281`

**What**: Test spawns a real `sleep 3` and relies on the host scheduler to fire the 1-second timeout before the sleep completes. Total wall-time per run >=1s and a slow CI runner could let the sleep complete or behave inconsistently. No deterministic sync point — purely sleep-based timing.

**Why it matters**: Slow tests + timing flakiness. flakiness-patterns calls out the small ratio between sleep and timeout as a known anti-pattern.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace real sleep with a deterministic blocking mechanism (e.g. a stdin-blocked cat or fifo-read) so timeout is the only termination cause
- [ ] #2 Or move timeout-path coverage to a unit test using injected time, leaving the integration test to verify CLI plumbing only
<!-- AC:END -->
