---
id: TASK-1230
title: >-
  TEST-15: canonical_workspace_cached burst test hardcodes /tmp and sleeps 20ms
  for synchronisation
status: To Do
assignee:
  - TASK-1266
created_date: '2026-05-08 12:58'
updated_date: '2026-05-08 13:19'
labels:
  - code-review-rust
  - tests
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/build.rs:802-848`

**What**: `canonical_workspace_cached_collapses_burst_to_single_canonicalize` builds keys under `PathBuf::from("/tmp/ops-task-1095-burst-...")` and inserts a `Duration::from_millis(20)` sleep inside the canonicalize closure to "reliably reach the write-lock re-check window". The test depends on /tmp existing (Windows) and on 20ms surviving CI scheduler jitter.

**Why it matters**: Wall-clock-coupled tests are flaky on heavy CI; hardcoded /tmp blocks portability. The test claims to pin the thundering-herd guarantee but only does so on Unix runners with predictable scheduling.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace /tmp with std::env::temp_dir() or tempfile
- [ ] #2 Use a Barrier or Notify rendezvous instead of sleep(20ms)
- [ ] #3 Or pin the property at the cache-API level via a closure that signals start/finish
<!-- AC:END -->
