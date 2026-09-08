---
id: TASK-2164
title: >-
  API-14: the ops-tokei public surface has no doc summaries, and its one
  fallible public helper documents only its errors
status: To Do
assignee:
  - TASK-2247
created_date: '2026-09-08 07:05'
updated_date: '2026-09-08 11:00'
labels:
  - code-review-rust
  - api-design
dependencies: []
modified_files:
  - extensions/tokei/src/lib.rs
  - extensions/tokei/src/ingestor.rs
  - extensions/tokei/src/views.rs
priority: low
ordinal: 77000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/tokei/src/lib.rs:37-42`, `extensions/tokei/src/ingestor.rs:11`, `extensions/tokei/src/views.rs:53-57`

**What**: every public item in the crate except `collect_tokei` and `tokei_languages_view_sql` is undocumented:

- `extensions/tokei/src/lib.rs:37` `pub const NAME`
- `extensions/tokei/src/lib.rs:38` `pub const DESCRIPTION`
- `extensions/tokei/src/lib.rs:39` `pub const SHORTNAME`
- `extensions/tokei/src/lib.rs:40` `pub const DATA_PROVIDER_NAME`
- `extensions/tokei/src/lib.rs:42` `pub struct TokeiExtension`
- `extensions/tokei/src/ingestor.rs:11` `pub struct TokeiIngestor` (re-exported at the crate root as `ops_tokei::TokeiIngestor`)
- `extensions/tokei/src/views.rs:57` `pub fn tokei_files_create_sql` — has a `# Errors` section but no summary line, so rustdoc renders it with an empty description

The crate root and `views` both carry module docs, so the gap is item-level only.

**Why it matters**: same as the sibling API-14 findings already filed for ops-about (TASK-2071), ops-extension (TASK-2098), ops-duckdb (TASK-2130), ops-git (TASK-2110) and config-checkers (TASK-2138) — a rustdoc page of bare signatures, and no way to turn on `#![warn(missing_docs)]` workspace-wide while the gaps exist. The four `NAME`/`DESCRIPTION`/`SHORTNAME`/`DATA_PROVIDER_NAME` constants are the same undocumented quartet flagged in ops-duckdb, so whatever wording that task settles on should be reused here.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every public item listed above carries a one-line doc summary
- [ ] #2 tokei_files_create_sql has a summary line above its # Errors section
- [ ] #3 Wording for the NAME/DESCRIPTION/SHORTNAME/DATA_PROVIDER_NAME quartet matches whatever the sibling ops-duckdb API-14 task (TASK-2130) settles on
<!-- AC:END -->
