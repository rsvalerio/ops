---
id: TASK-2410
title: 'API-14: Public ops-sqlite items and fields lack doc summaries'
status: Done
assignee: []
created_date: '2026-10-04 14:18'
updated_date: '2026-10-04 15:28'
labels:
  - code-review-rust
  - API
dependencies: []
parent_task_id: 'TASK-2416'
modified_files:
  - extensions/sqlite/src/schema.rs
  - extensions/sqlite/src/ingestor.rs
  - extensions/sqlite/src/error.rs
  - extensions/sqlite/src/sql/validation.rs
priority: low
ordinal: 1000
dedup_key: 'API-14:extensions/sqlite/src/schema.rs:DataSourceMetadata'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/sqlite/src/schema.rs:75-135`, `extensions/sqlite/src/ingestor.rs:55-66`, `extensions/sqlite/src/error.rs:~85`, `extensions/sqlite/src/sql/validation.rs:~207` (`validate_no_traversal`)

**What**: Undocumented public items: `SourceName` and `WorkspaceRoot` (types plus `new` / `as_str` / `as_os_str`), the pub fields of `DataSourceMetadata` (`source_name`, `workspace_root`, `source_path`, `record_count`, `checksum`), `DataSourceMetadata::new`, the pub fields `name` and `json_filename` of `SidecarIngestorConfig`, the fields of `DbError::Timeout { label, timeout_secs }`, and `validate_no_traversal`, whose doc has only a `# Errors` section and no summary sentence.

**Why it matters**: These are re-exported public API (`pub use schema::...`), and the rest of the crate documents every field. The summary sentence is mandatory under API-14.

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Every public item and public field in the listed locations has a doc summary
- [x] #2 validate_no_traversal gains a summary sentence ahead of # Errors

<!-- AC:END -->
