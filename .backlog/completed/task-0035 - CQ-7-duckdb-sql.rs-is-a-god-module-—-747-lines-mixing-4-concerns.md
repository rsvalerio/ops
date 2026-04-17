---
id: TASK-0035
title: 'CQ-7: duckdb/sql.rs is a god module — 747 lines mixing 4+ concerns'
status: Done
assignee: []
created_date: '2026-04-14 19:41'
updated_date: '2026-04-15 09:56'
labels:
  - rust-code-quality
  - architecture
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
extensions/duckdb/src/sql.rs — 747 non-test lines (ARCH-1 threshold: 500) containing 4+ unrelated concerns: (1) path validation/security (validate_path_chars, validate_no_traversal, sanitize_path_for_sql, escape_sql_string), (2) table operations (create_table_from_json_sql, table_exists, table_has_data, drop_table_if_exists), (3) sidecar file I/O (write/read/remove_workspace_sidecar, checksum_file), (4) domain-specific queries (query_project_loc, query_crate_coverage, etc.). Rules: ARCH-1, ARCH-3. Suggested split: sql/path_security.rs (concern 1), sql/schema.rs (concerns 2), sql/sidecar.rs (concern 3), keep domain queries in sql.rs or sql/query.rs.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 sql.rs non-test code ≤500 lines
- [ ] #2 Path validation and sidecar I/O extracted to separate modules
<!-- AC:END -->
