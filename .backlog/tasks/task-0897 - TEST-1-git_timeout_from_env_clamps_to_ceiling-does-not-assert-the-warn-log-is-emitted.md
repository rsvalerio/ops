---
id: TASK-0897
title: >-
  TEST-1: git_timeout_from_env_clamps_to_ceiling does not assert the warn-log is
  emitted
status: Triage
assignee: []
created_date: '2026-05-02 09:47'
labels:
  - code-review-rust
  - test
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: extensions/run-before-commit/src/lib.rs:564

**What**: The new test (added with TASK-0783) verifies the returned Duration is clamped but does not assert the tracing::warn event is emitted. The clamping diagnostic is part of the contract (per the rationale: operator must learn that the env var was misconfigured) yet is untested.

**Why it matters**: A future refactor could silently drop the warn! while preserving the clamp. The clamp would still test green, but operators would lose the only signal that their env var was rejected.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Use a tracing-test subscriber (or capture_tracing helper) to assert exactly one WARN event with fields env=TIMEOUT env var name, requested_secs, ceiling_secs
- [ ] #2 Add a parallel test asserting NO warn is emitted at the boundary (n == MAX_GIT_TIMEOUT_SECS)
- [ ] #3 Both tests run under the project standard test setup (no extra global subscriber state leak)
<!-- AC:END -->
