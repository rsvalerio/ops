---
id: TASK-0135
title: 'FN-3: upsert_data_source has 6 parameters (allow-annotated)'
status: Done
assignee: []
created_date: '2026-04-22 21:16'
updated_date: '2026-04-23 08:34'
labels:
  - rust-code-review
  - fn
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/schema.rs:51-57`

**What**: `upsert_data_source` takes 6 parameters (db, source_name, workspace_root, source_path, record_count, checksum) and silences clippy::too_many_arguments with an allow attribute — the allow is the signal that the API already felt wrong.

**Why it matters**: Long positional arg lists are easy to mis-order (two &str positional args especially); grouping related fields into a struct gives self-documenting call sites.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Group source_name/workspace_root/source_path/record_count/checksum into a DataSourceMetadata struct
- [x] #2 Remove #[allow(clippy::too_many_arguments)]
<!-- AC:END -->
