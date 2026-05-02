---
id: TASK-0912
title: >-
  API-2: DataSourceMetadata::new takes two adjacent positional &str args
  (source_name, workspace_root)
status: Done
assignee: []
created_date: '2026-05-02 10:11'
updated_date: '2026-05-02 11:24'
labels:
  - code-review-rust
  - api
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/schema.rs:60`

**What**: DataSourceMetadata::new takes source_name and workspace_root as two adjacent &str positional parameters. Both are silently swappable; workspace_root is half of the (source_name, workspace_root) primary key in data_sources.

**Why it matters**: An argument-order swap silently writes rows under the wrong primary key, producing duplicate ingest records, lost upserts, and divergent checksums that future runs cannot reconcile.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 source_name and workspace_root are wrapped in distinct newtypes so a swap is a compile error
- [ ] #2 All call sites in extensions and extensions-rust are migrated to the newtypes
<!-- AC:END -->
