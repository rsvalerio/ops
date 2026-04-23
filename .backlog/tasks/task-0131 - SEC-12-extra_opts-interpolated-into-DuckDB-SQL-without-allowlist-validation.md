---
id: TASK-0131
title: 'SEC-12: extra_opts interpolated into DuckDB SQL without allowlist validation'
status: Done
assignee: []
created_date: '2026-04-22 21:16'
updated_date: '2026-04-23 08:31'
labels:
  - rust-code-review
  - sec
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/ingest.rs:12-27` (create_table_from_json_sql)

**What**: The `extra_opts` argument is string-interpolated directly into `read_json_auto('{path}', {opts})`. Path is escaped, opts are not. A caller passing `maximum_object_size=1, injection='...')  --` could break the statement boundary.

**Why it matters**: Defense-in-depth: even with trusted callers today, SEC-12 requires unvalidated SQL fragments to be rejected. Future refactors could expose this to user-controlled input.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Validate extra_opts with a whitelist (e.g., alphanumeric + underscores + '=' + ',' + digits only) or accept a typed struct/list of (key, value) pairs
- [x] #2 Add a test asserting that malicious extra_opts (containing quotes, semicolons, or parentheses) is rejected
<!-- AC:END -->
