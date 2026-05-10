---
id: TASK-0654
title: >-
  ERR-1: read_conf_d_files silently drops failed read_dir entries via
  filter_map(.ok())
status: Done
assignee:
  - TASK-0737
created_date: '2026-04-30 05:12'
updated_date: '2026-04-30 17:47'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/loader.rs:150-157`

**What**: `read_conf_d_files` swallows `read_dir` entry errors with `filter_map(|e| e.ok())`. A single broken inode under `.ops.d/` (permission-denied, deleted-mid-iteration) makes that overlay invisible without any signal.

**Why it matters**: Sibling `merge_conf_d` is explicit about hard-erroring on parse failures; entry enumeration should be at least as loud or operators debugging "my overlay isn't applying" get no log line.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Log entry-iteration errors at tracing::warn! with the directory path so the loss is auditable
- [ ] #2 Consider returning anyhow::Result<Vec<PathBuf>> and propagating, matching merge_conf_d's fail-loud policy
<!-- AC:END -->
