---
id: TASK-1095
title: >-
  PERF-3: canonical_workspace_cached re-canonicalizes after write-lock
  acquisition without re-checking entry
status: Done
assignee: []
created_date: '2026-05-07 21:32'
updated_date: '2026-05-07 23:36'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/build.rs:33-48`

**What**: After the read-lock miss, the writer path always re-canonicalizes and inserts even if a racing writer already populated the entry. Under burst startup this is N parallel `canonicalize` syscalls for the same path on the blocking pool. Distinct from TASK-1063 (which covers unboundedness) — this is the residual thundering-herd after TASK-0839's contention fix.

**Why it matters**: N processes/threads doing first-time canonicalize of the same workspace path each pay full syscall cost instead of one.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 After acquiring the write lock, re-check the entry and skip the canonicalize call on a hit
- [ ] #2 A microbenchmark (or tracing::trace! count) shows at most one canonicalize call per workspace path under N=32 concurrent first-callers
- [ ] #3 Existing CONC-7 read-lock fast-path stays unchanged
<!-- AC:END -->
