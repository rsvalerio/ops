---
id: TASK-0551
title: 'ERR-1: write_new_hook does not fsync, but the legacy-upgrade temp path does'
status: Done
assignee:
  - TASK-0645
created_date: '2026-04-29 05:02'
updated_date: '2026-04-29 17:43'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/install.rs:49`

**What**: write_new_hook writes the hook script and drops the file without sync_all, while the sibling write_temp_hook (used by the legacy-upgrade path) calls tmp.sync_all(). A crash between the install step and the next git invocation can leave a zero-byte .git/hooks/<filename> after first install — the very state the upgrade path was hardened against (SEC-25).

**Why it matters**: Asymmetric durability between two install paths to the same destination; the rationale in upgrade_legacy_hook ("close the read/write TOCTOU window") applies equally to a fresh install on a system that crashes before fsync.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 write_new_hook calls file.sync_all() before logging Installed hook
- [ ] #2 A comment documents the parity with write_temp_hook
<!-- AC:END -->
