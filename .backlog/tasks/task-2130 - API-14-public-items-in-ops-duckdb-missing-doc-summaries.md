---
id: TASK-2130
title: 'API-14: public items in ops-duckdb missing doc summaries'
status: To Do
assignee:
  - TASK-2247
created_date: '2026-09-08 06:55'
updated_date: '2026-09-08 10:59'
labels:
  - code-review-rust
  - api-design
dependencies: []
modified_files:
  - extensions/duckdb/src/lib.rs
  - extensions/duckdb/src/error.rs
  - extensions/duckdb/src/ingestor.rs
  - extensions/duckdb/src/sql/ingest/sql.rs
  - extensions/duckdb/src/sql/query/helpers.rs
  - extensions/duckdb/src/sql/query/loc.rs
  - extensions/duckdb/src/sql/validation.rs
priority: low
ordinal: 46000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/lib.rs:94`

**What**: candidate list of `pub` items with no doc summary (the crate is a
library, so these are all API surface):

* `extensions/duckdb/src/lib.rs:94-97` — `NAME`, `DESCRIPTION`, `SHORTNAME`,
  `DATA_PROVIDER_NAME`. The comment above them explains why they carry no
  `#[allow(dead_code)]`; none of them says what the constant is.
* `extensions/duckdb/src/lib.rs:105` — `pub struct DuckDbExtension` (no summary).
* `extensions/duckdb/src/lib.rs:111` — `DuckDbExtension::new` (no summary).
* `extensions/duckdb/src/error.rs:85` — `DbError::query_failed` (no summary).
* `extensions/duckdb/src/error.rs:22-40` — the `DuckDb`, `Io`, `QueryFailed`,
  `Serialization`, `RecordCountOverflow`, `InvalidRecordCount`, `NonUtf8Path`
  and `SqlValidation` variants of the public `DbError` enum carry only their
  `#[error(...)]` string; the neighbouring `MutexPoisoned`, `NotFileBacked`,
  `Timeout` and `External` variants are documented, so the gap is visible
  inside one enum.
* `extensions/duckdb/src/ingestor.rs:38` — `LoadResult::success` (no summary).
* `extensions/duckdb/src/sql/ingest/sql.rs:67` —
  `CreateViewSql::create_or_replace` has a body paragraph but no leading
  one-line summary (rustdoc takes the first line of the "The gate is …"
  paragraph as the summary).
* `extensions/duckdb/src/sql/query/helpers.rs:52-56` — `CrateCoverage`'s three
  public fields, and `CrateCoverage::new` / `zero` at 60/69.
* `extensions/duckdb/src/sql/query/loc.rs:153-161` — `RustLocStat`'s seven
  public fields (the type doc explains the design but never says what `docs`
  vs `comments` vs `lines` mean, which is the exact confusion the type doc
  says field names are there to prevent).
* `extensions/duckdb/src/sql/validation.rs:42-53` — the six `SqlError` variants.

**Why it matters**: this crate is consumed by five sibling extension crates
(`extensions/tokei`, `extensions-rust/loc`, `extensions-rust/test-coverage`,
`extensions-rust/metadata`, `extensions-rust/about`) through exactly these
types. Sibling findings TASK-2071 (ops-about), TASK-2097 (ops-core) and
TASK-2098 (ops-extension) cover the same rule on the neighbouring crates.

<!-- scan confidence: candidates to inspect; each verified by reading, test code excluded -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 every public item listed above carries a one-line doc summary saying what it is, not why a lint was suppressed
- [ ] #2 RustLocStat and CrateCoverage fields state their unit and whether regions overlap, so a caller can tell docs/comments/lines apart
- [ ] #3 CreateViewSql::create_or_replace gains a leading summary line ahead of its rationale paragraph
<!-- AC:END -->
