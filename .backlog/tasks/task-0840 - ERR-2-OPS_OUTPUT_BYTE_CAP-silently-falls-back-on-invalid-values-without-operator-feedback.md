---
id: TASK-0840
title: >-
  ERR-2: OPS_OUTPUT_BYTE_CAP silently falls back on invalid values without
  operator feedback
status: Triage
assignee: []
created_date: '2026-05-02 09:14'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/results.rs:124-130`

**What**: output_byte_cap() does std::env::var(OUTPUT_CAP_ENV).ok().and_then(|s| s.parse::<usize>().ok()).filter(|&n| n > 0).unwrap_or(DEFAULT_OUTPUT_BYTE_CAP). A user who sets OPS_OUTPUT_BYTE_CAP=foo, OPS_OUTPUT_BYTE_CAP=0, or OPS_OUTPUT_BYTE_CAP=-1 gets the 4 MiB default with zero diagnostic - they cannot tell whether their env var "took".

**Why it matters**: For an env knob whose explicit purpose is to override the cap, a silent fallback hides operator intent and produces confusing CI behaviour. Mirrors the rationale already followed for OPS_LOG_LEVEL in cli/src/main.rs:96-101.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 On parse-failure or non-positive value, emit a single tracing::warn! (one-shot via OnceLock) naming the offending value and the fallback
- [ ] #2 Continue to fall back to DEFAULT_OUTPUT_BYTE_CAP so production behaviour is unchanged
- [ ] #3 Unit test that an invalid value produces the warn message and the default cap
<!-- AC:END -->
