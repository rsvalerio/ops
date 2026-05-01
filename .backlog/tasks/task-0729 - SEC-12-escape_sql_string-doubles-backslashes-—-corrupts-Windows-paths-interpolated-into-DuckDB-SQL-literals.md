---
id: TASK-0729
title: >-
  SEC-12: escape_sql_string doubles backslashes — corrupts Windows paths
  interpolated into DuckDB SQL literals
status: Done
assignee:
  - TASK-0743
created_date: '2026-04-30 05:48'
updated_date: '2026-04-30 20:30'
labels:
  - code-review-rust
  - SEC
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/validation.rs:109-120` (escape rule); `extensions/duckdb/src/sql/validation.rs:172-180` (call site `prepare_path_for_sql`)

**What**: `escape_sql_string` maps `\\` → `\\\\` (each backslash gets doubled). DuckDB SQL literals use SQL-standard semantics by default (no `E''` prefix), where backslash is a literal character — only `'` requires escaping (as `''`). Doubling backslashes therefore *produces* an extra backslash in the path DuckDB sees, rather than preserving the original. On Unix the impact is masked because `validate_path_chars` rejects `\\` outright (Unix branch of the `cfg!(windows)` guard), but on Windows the validator accepts `C:\Users\file.json`, the escaper rewrites it to `C:\\Users\\file.json`, and DuckDB interprets that as `C:\\Users\\file.json` — a path that does not exist. Tests at validation.rs:198-200 explicitly pin this incorrect doubling.

For SQL injection defence: backslash *is* relevant in DuckDB only inside `E'…'` literals, which this codebase never emits. Doubling is therefore both unnecessary and wrong for the only mode we use.

**Why it matters**: Windows users get "file not found" errors for any path containing a backslash, which is every absolute Windows path. The bug also constitutes a correctness divergence between platforms where the safety story relies on identical sanitization. Fix: drop the `\\` arm from `escape_sql_string` (keep `'` doubling and `\0` removal), and update the regression test that pins the broken behaviour.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 escape_sql_string preserves a single backslash unchanged (only \\' and \0 receive special handling)
- [x] #2 Windows path round-trip test (e.g. C:\\Users\\file.json) goes through prepare_path_for_sql and yields a string DuckDB can open
<!-- AC:END -->
