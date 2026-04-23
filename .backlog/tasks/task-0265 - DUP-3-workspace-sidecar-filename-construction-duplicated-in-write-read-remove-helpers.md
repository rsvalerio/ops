---
id: TASK-0265
title: >-
  DUP-3: workspace sidecar filename construction duplicated in write/read/remove
  helpers
status: Done
assignee: []
created_date: '2026-04-23 06:36'
updated_date: '2026-04-23 09:19'
labels:
  - rust-code-review
  - duplication
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/ingest.rs:109`

**What**: Same `{name}_workspace.txt` format string appears 3x — any rename breaks only two silently.

**Why it matters**: Drift-prone; caught only in integration.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Extract fn sidecar_path(data_dir, name) -> PathBuf used by all three
- [x] #2 Update callers
<!-- AC:END -->
