---
id: TASK-2373
title: 'READ-12: positional-formatted tracing messages in log_manifest_load_failure'
status: To Do
assignee: []
created_date: '2026-10-04 14:11'
updated_date: '2026-10-04 14:52'
labels:
  - code-review-rust
  - READ
dependencies: []
parent_task_id: 'TASK-2418'
modified_files:
  - extensions-rust/about/src/manifest.rs
  - extensions-rust/about/src/members.rs
  - extensions-rust/about/src/units.rs
priority: low
ordinal: 1000
dedup_key: 'READ-12:extensions-rust/about/src/manifest.rs:log_manifest_load_failure'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/manifest.rs:~191`

**What**: `log_manifest_load_failure` interpolates the error into the message text (`"failed to load workspace Cargo.toml: {err:#}"` and the debug-level sibling) instead of recording it as a named field. Also, several runtime log messages are prefixed with internal rule/task IDs (`"SEC-14 / TASK-1246: rejecting ..."` in members.rs `member_path_is_workspace_safe_or_warn`, `"ERR-2 / TASK-1253: no dep_count row ..."` and `"PERF-3 / TASK-1570: ..."` in units.rs `resolve_dep_count`, `"TASK-1204: ..."` in manifest.rs `resolve_workspace_root`), which bakes review bookkeeping into the message template and defeats grouping on a stable message.

**Why it matters**: READ-12: the error cannot be filtered or aggregated as a field, and message strings that embed task IDs are not stable templates. Other sites in the crate already use `error = ?e` fields.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 log_manifest_load_failure records the error as a field (error = %err / ?err) with a constant message
- [ ] #2 Log messages no longer carry rule/task ID prefixes; IDs remain in code comments if wanted
<!-- AC:END -->
