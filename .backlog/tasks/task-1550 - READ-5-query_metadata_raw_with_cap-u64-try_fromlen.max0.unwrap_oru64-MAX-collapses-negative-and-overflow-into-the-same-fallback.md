---
id: TASK-1550
title: >-
  READ-5: query_metadata_raw_with_cap
  u64::try_from(len.max(0)).unwrap_or(u64::MAX) collapses negative and overflow
  into the same fallback
status: To Do
assignee:
  - TASK-1576
created_date: '2026-05-19 15:27'
updated_date: '2026-05-19 16:46'
labels:
  - code-review-rust
  - READ
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/lib.rs:263`

**What**: `let len = u64::try_from(len.max(0)).unwrap_or(u64::MAX);` first clamps a possibly-negative DuckDB `i64` to ≥0 via `.max(0)`, then converts to `u64`. After the clamp, `i64::try_from -> u64` is infallible for any non-negative `i64` (range fits), so the `.unwrap_or(u64::MAX)` arm is unreachable. The line conflates two thoughts: defensive handling of a negative `octet_length` and overflow-fallback that can never trigger.

**Why it matters**: READ-5 covers code whose comment-shape implies a danger the code cannot actually produce. A reviewer reading this line has to mentally model both arms; the inference is "negative `octet_length` is possible (defensive `.max(0)`) AND the conversion can overflow (defensive `.unwrap_or(u64::MAX)`)". Only the first is true. Rewriting as `let len = u64::try_from(len).unwrap_or(0);` (treat any DuckDB-reported negative or sentinel as zero-length, which trips no cap) or `let len = len.try_into().unwrap_or(0_u64);` is shorter, drops the dead arm, and makes the actual policy obvious. If the author wanted u64::MAX as a tripwire (so an unexpected negative deliberately fails the cap), keep it but document that intent.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The len-coercion line either has a single, unambiguous fallback or carries a comment explaining the dual-arm intent
- [ ] #2 The behaviour on a negative octet_length is pinned by a test (today no path reaches the .unwrap_or arm; the test asserts the policy)
<!-- AC:END -->
