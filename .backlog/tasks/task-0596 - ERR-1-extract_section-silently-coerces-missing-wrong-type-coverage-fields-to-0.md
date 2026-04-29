---
id: TASK-0596
title: >-
  ERR-1: extract_section silently coerces missing/wrong-type coverage fields to
  0
status: Triage
assignee: []
created_date: '2026-04-29 05:18'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/lib.rs:127`

**What**: extract_section reads count/covered/notcovered/percent with `.and_then(as_i64).unwrap_or(0)` — type drift in llvm-cov (e.g. count becoming string/float, or section omitted) silently yields zeros. Same anti-pattern as TASK-0506 (negative-i64 coercion) but on coverage ingest; no tracing::warn distinguishes "absent section" from "field present but unparseable".

**Why it matters**: A schema change makes coverage display as 0% with no log signal — exactly the regression class TASK-0376/ERR-2 is meant to prevent.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Field-shape mismatches (Some(non-i64)) emit tracing::warn once per provide() invocation
- [ ] #2 Missing section path remains silent (legitimately empty)
<!-- AC:END -->
