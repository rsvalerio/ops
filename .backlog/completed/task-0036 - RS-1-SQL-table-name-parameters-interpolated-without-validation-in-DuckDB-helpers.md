---
id: TASK-0036
title: >-
  RS-1: SQL table-name parameters interpolated without validation in DuckDB
  helpers
status: Done
assignee: []
created_date: '2026-04-14 19:44'
updated_date: '2026-04-15 09:56'
labels:
  - rust-security
  - defense-in-depth
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
create_table_from_json_sql() in extensions/duckdb/src/sql.rs:89 interpolates table_name and extra_opts directly into SQL via format!() without validation or escaping. SidecarIngestorConfig::load_with_sidecar() in extensions/duckdb/src/ingestor.rs:77 does the same with self.count_table. All current call sites pass hardcoded string literals (&'static str), so there is no exploitable path today. However, the path parameter in the same function IS validated via prepare_path_for_sql(), creating an inconsistency. Defense-in-depth improvement: validate table names against an identifier pattern (e.g., [a-zA-Z_][a-zA-Z0-9_]*) and quote them consistently. OWASP: A03 (Injection). SEC-12.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 table_name parameters in create_table_from_json_sql and SidecarIngestorConfig methods are validated against a safe identifier pattern before SQL interpolation
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-04-14 re-audit: SidecarIngestorConfig::load_with_sidecar (ingestor.rs:77) also interpolates self.count_table into SQL without validation. Same risk profile — count_table is &'static str from compile-time config, not user input. Should be included in the same fix.

2026-04-14 re-audit: create_table_from_json_sql() also interpolates extra_opts (line 97) directly into SQL without validation. Current single call site passes a hardcoded literal ("maximum_object_size=67108864" in metadata/views.rs:33). Same risk profile — defense-in-depth improvement: validate extra_opts format or use a structured options type.
<!-- SECTION:NOTES:END -->
