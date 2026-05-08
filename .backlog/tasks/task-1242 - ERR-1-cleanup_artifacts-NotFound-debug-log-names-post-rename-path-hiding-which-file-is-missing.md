---
id: TASK-1242
title: >-
  ERR-1: cleanup_artifacts NotFound debug log names post-rename path, hiding
  which file is missing
status: To Do
assignee:
  - TASK-1268
created_date: '2026-05-08 12:59'
updated_date: '2026-05-08 13:19'
labels:
  - code-review-rust
  - errors
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/ingestor.rs:286-305`

**What**: After `std::fs::rename(json_path, &done_path)` succeeds, the local `json_path` binding is rebound to `done_path.as_path()`. The subsequent NotFound debug breadcrumb emits `path = ?json_path.display()` from this post-rename binding, so an operator chasing a half-cleaned crash sees the *.json.done name even when the rename fell back (cross-device) and the original *.json is what disappeared.

**Why it matters**: TASK-1008 documents the rename → unlink ordering specifically so debris is unambiguously identifiable. The current breadcrumb undermines that by collapsing the rename-fallback case into the same log line.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Capture both original_json_path and effective post-rename path; log both fields on NotFound
- [ ] #2 Regression test asserting both names appear in the breadcrumb on cross-device fallback
- [ ] #3 Update the doc to spell out the dual-path logging contract
<!-- AC:END -->
