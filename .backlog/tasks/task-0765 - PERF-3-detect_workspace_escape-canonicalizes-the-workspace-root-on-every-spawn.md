---
id: TASK-0765
title: >-
  PERF-3: detect_workspace_escape canonicalizes the workspace root on every
  spawn
status: Triage
assignee: []
created_date: '2026-05-01 05:55'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/build.rs:90-96`

**What**: detect_workspace_escape calls std::fs::canonicalize(workspace).ok() for every command spawn even though the workspace path is fixed for the runner's lifetime (CommandRunner holds it in Arc<PathBuf>).

**Why it matters**: Each canonicalize is a syscall chain; under MAX_PARALLEL=32 spawn rate this is wasted work on the blocking pool. The joined-path canonicalize is necessary; the workspace canonicalize is redundant after the first call.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Cache the canonical workspace path on CommandRunner (compute lazily once via OnceLock or at construction) and pass it into detect_workspace_escape
- [ ] #2 Keep the joined-path canonicalize per call (its target legitimately changes)
- [ ] #3 Pin behavioural parity with a test that verifies escape detection still fires on symlinked-into-workspace paths
<!-- AC:END -->
