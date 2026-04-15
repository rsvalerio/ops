---
id: TASK-0058
title: 'RS-4: i64-to-usize cast on COUNT(*) result in query_dependency_count'
status: Done
assignee: []
created_date: '2026-04-14 20:50'
updated_date: '2026-04-15 09:56'
labels:
  - rust-security
  - defense-in-depth
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
In extensions/duckdb/src/sql.rs:434, query_dependency_count() casts a DuckDB COUNT(*) result (i64) to usize via 'as usize'. While COUNT(*) cannot return negative values in practice, the 'as' cast would silently truncate or wrap in release builds (SEC-15). On 32-bit platforms, an i64 exceeding usize::MAX would also truncate. Defense-in-depth: use try_from or .max(0) as usize. Affected crate: ops-duckdb. OWASP: A04 (Insecure Design). SEC-15.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 i64-to-usize conversion in query_dependency_count uses checked arithmetic (try_from or explicit clamp) instead of bare 'as' cast
<!-- AC:END -->
