---
id: TASK-1618
title: >-
  API-9: test-coverage views module and SQL helpers are pub with no external
  callers
status: Done
assignee:
  - TASK-1635
created_date: '2026-05-22 06:53'
updated_date: '2026-05-22 10:12'
labels:
  - code-review-rust
  - api
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/lib.rs:22`, `extensions-rust/test-coverage/src/views.rs:12,16`

**What**: `lib.rs` declares `pub mod views;` and `views.rs` exposes `pub fn coverage_files_create_sql` and `pub fn coverage_summary_view_sql`. A workspace-wide grep finds no external caller — both functions are used only by `crate::ingestor::CoverageIngestor::load`. The same applies to the `views` module itself.

Candidates (verified internal-only):
- `views.rs:12` `pub fn coverage_files_create_sql(path: &Path) -> Result<String, SqlError>` — only call site is `ingestor.rs:26`
- `views.rs:16` `pub fn coverage_summary_view_sql() -> String` — only call site is `ingestor.rs:27`
- `lib.rs:22` `pub mod views;` — no `use ops_test_coverage::views` / `test_coverage::views` matches anywhere in the workspace

**Why it matters**: API minimisation (API-9). Public visibility is a contract — downstream crates can start to depend on these signatures, making future changes (e.g. inlining the view DDL, changing the SqlError type, dropping a helper) a SemVer event for a crate marked `publish = false`. Tightening to `pub(crate)` keeps the surface honest and lets the compiler enforce that the views layer remains an implementation detail of `CoverageIngestor`. Sibling task TASK-1601 / TASK-1602 already track over-public surface for `CoverageIngestor` and the parse helpers; this is the same pattern in the third module.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either: change 'pub mod views;' to 'mod views;' and downgrade both pub fns in views.rs to pub(crate), OR document an external caller (test binary, integration crate) that requires the public surface
- [ ] #2 After the change, 'cargo check -p ops-test-coverage --all-targets' still passes; ingestor.rs still resolves views::coverage_files_create_sql / views::coverage_summary_view_sql
- [ ] #3 If kept pub for a documented reason, add a module-level //! comment in views.rs naming the external caller so future reviewers do not relitigate
<!-- AC:END -->
