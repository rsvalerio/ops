---
id: TASK-2356
title: 'READ-13: resolve_global_config_path doc still describes a OnceLock initialiser'
status: To Do
assignee: []
created_date: '2026-10-04 14:09'
updated_date: '2026-10-04 14:52'
labels:
  - code-review-rust
  - READ
dependencies: []
parent_task_id: 'TASK-2423'
modified_files:
  - crates/core/src/config/loader/global.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:crates/core/src/config/loader/global.rs:resolve_global_config_path'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/loader/global.rs:~151`

**What**: The doc on `resolve_global_config_path` says it is "invoked exactly once by the `GLOBAL_CONFIG_PATH` `OnceLock` initialiser". `GLOBAL_CONFIG_PATH` is now a `RwLock<Option<Option<PathBuf>>>` (see lines 60-68) and the `reset_global_config_path_cache` hook (test-support) makes it run more than once. The doc is stale and misdescribes the call contract; the same file's `GlobalConfigPathResetToken` docs show the migration happened.

**Why it matters**: Readers relying on "exactly once" (e.g. for the one-shot `tracing::debug` breadcrumb) get the wrong model.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 resolve_global_config_path docs describe the RwLock-backed cache, including that the reset hook re-invokes it
<!-- AC:END -->
