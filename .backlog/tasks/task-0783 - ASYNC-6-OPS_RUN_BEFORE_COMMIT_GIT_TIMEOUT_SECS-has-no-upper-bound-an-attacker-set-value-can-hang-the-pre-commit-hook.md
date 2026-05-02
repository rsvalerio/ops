---
id: TASK-0783
title: >-
  ASYNC-6: OPS_RUN_BEFORE_COMMIT_GIT_TIMEOUT_SECS has no upper bound; an
  attacker-set value can hang the pre-commit hook
status: Done
assignee:
  - TASK-0827
created_date: '2026-05-01 05:58'
updated_date: '2026-05-02 07:27'
labels:
  - code-review-rust
  - async
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-commit/src/lib.rs:88`

**What**: git_timeout_from_env parses any non-zero u64 from the environment. A .envrc/CI-injected OPS_RUN_BEFORE_COMMIT_GIT_TIMEOUT_SECS=999999999 reverts the bounded-wait fix from TASK-0589 to "essentially unbounded".

**Why it matters**: Mirrors TASK-0304 (OPS_SUBPROCESS_TIMEOUT_SECS upper bound) — same threat model. The whole point of the bounded wait is that the developer's commit cannot park indefinitely.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Cap the parsed value at a reasonable ceiling (e.g., 300 s) and warn-and-clamp when exceeded, matching the policy from TASK-0304
- [ ] #2 Test verifies an overlarge value is clamped, not honoured
<!-- AC:END -->
