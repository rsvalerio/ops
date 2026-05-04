---
id: TASK-1003
title: >-
  ERR-5: tokei views::tokei_languages_view_sql() expect on static idents in test
  code masked by helper, but pre-prod call returns Result
status: Triage
assignee: []
created_date: '2026-05-04 22:03'
labels:
  - code-review-rust
  - API
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/tokei/src/views.rs:20-29`

**What**: `tokei_languages_view_sql() -> Result<String, SqlError>` builds two `quoted_ident("tokei_languages")?` / `quoted_ident("tokei_files")?` expressions over compile-time-known string literals. Both literals are valid identifiers; the `Result` return is structurally unreachable at runtime, but the function still *propagates* the error to every caller. As a result, every consumer (the ingestor `load_with_sidecar` invocation) must handle a `Result` whose `Err` variant cannot occur.

The sister type `TableName::from_static` (extensions/duckdb/src/sql/validation.rs:78-106) already solved exactly this problem with a const-validated newtype that fails the build on an invalid literal and exposes only an infallible `quoted()` accessor. `tokei_languages_view_sql` predates the newtype and now drags an unnecessary error path through the call chain.

**Why it matters**:
- API-1 / clarity: the function's Result type lies about its failure modes. Future maintainers reading the signature add defensive handling that can never fire, raising cognitive load (CL-*) and obscuring the real failure budget.
- Consistency: `tokei_files_create_sql` legitimately returns Result (the path is runtime data), but `tokei_languages_view_sql` over only compile-time idents should not. The mixed posture invites a future refactor that "harmonises" by adding runtime validation back, undoing the const-fn invariant `TableName::from_static` already provides.
- Precedent: `SidecarIngestorConfig::count_table` (ingestor.rs:51) is the project's blessed const-time pattern. Tokei views should adopt it.

**Recommended fix**: compute the SQL via two `TableName::from_static("tokei_languages")` / `TableName::from_static("tokei_files")` and return `String` (no Result). This deletes the `expect("static idents must validate")` in the tests at lines 39 and 51 along with the dead error path.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 tokei_languages_view_sql returns String (or const &'static str) without a Result wrapper.
- [ ] #2 Implementation uses TableName::from_static for both identifiers, matching SidecarIngestorConfig precedent.
- [ ] #3 Callers no longer pattern-match on a Result that cannot fail; the test-side .expect lines disappear.
<!-- AC:END -->
