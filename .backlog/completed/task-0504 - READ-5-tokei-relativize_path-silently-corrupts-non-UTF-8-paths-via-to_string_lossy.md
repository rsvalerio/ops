---
id: TASK-0504
title: >-
  READ-5: tokei relativize_path silently corrupts non-UTF-8 paths via
  to_string_lossy
status: Done
assignee:
  - TASK-0534
created_date: '2026-04-28 06:50'
updated_date: '2026-04-28 18:57'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/tokei/src/lib.rs:148`

**What**: relativize_path calls to_string_lossy().into_owned() on the path that becomes the `file` field of every tokei row, replacing invalid bytes with U+FFFD. The duckdb crate elsewhere (sql/validation.rs prepare_path_for_sql, schema.rs upsert_data_source) explicitly rejects non-UTF-8 paths.

**Why it matters**: The corrupted path lands in the tokei_files table where it is joined via starts_with(file, member_path || '/') — coverage/LOC attribution silently misses files whose names round-tripped through U+FFFD. Inconsistent with the surrounding NonUtf8Path strict policy.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either return a Result and reject non-UTF-8, or document the lossy contract and add a test covering the lossy outcome
- [ ] #2 Match the policy used by upsert_data_source (DbError::NonUtf8Path)
<!-- AC:END -->
