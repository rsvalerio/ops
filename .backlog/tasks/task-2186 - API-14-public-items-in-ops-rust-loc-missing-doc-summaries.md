---
id: TASK-2186
title: 'API-14: public items in ops-rust-loc missing doc summaries'
status: To Do
assignee:
  - TASK-2247
created_date: '2026-09-08 07:14'
updated_date: '2026-09-08 11:00'
labels:
  - code-review-rust
  - api-design
dependencies: []
modified_files:
  - extensions-rust/loc/src/lib.rs
  - extensions-rust/loc/src/counter.rs
  - extensions-rust/loc/src/views.rs
  - extensions-rust/loc/src/ingestor.rs
priority: low
ordinal: 99000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/loc/src/lib.rs`, `extensions-rust/loc/src/counter.rs`, `extensions-rust/loc/src/views.rs`, `extensions-rust/loc/src/ingestor.rs`

**What**: The crate documents its hard parts extremely well (the walk policy, the size cap, the nesting cap), but a set of ordinary public items carry no doc comment at all, and two carry a comment that never states what the item *is*.

No doc comment:

- `lib.rs:36` `pub const NAME`
- `lib.rs:37` `pub const DESCRIPTION`
- `lib.rs:38` `pub const SHORTNAME`
- `lib.rs:39` `pub const DATA_PROVIDER_NAME`
- `lib.rs:41` `pub struct RustLocExtension`
- `ingestor.rs:10` `pub struct RustLocIngestor` (re-exported at the crate root as `pub use ingestor::RustLocIngestor`, so this is a root-level public type with no docs)
- `counter.rs:65-70` `pub enum LineKind` variants (`Blank`, `Comment`, `Doc`, `Code`) — the enum has docs, the variants do not
- `counter.rs:74-78` `pub enum Region` variants (`Main`, `Test`, `Example`)
- `counter.rs:82` `pub const fn Region::as_str`
- `counter.rs:116` `pub const fn Locs::is_empty`
- `counter.rs:94-98`, `counter.rs:137-140` the public fields of `Locs` and `FileCounts`

Comment present, summary missing:

- `views.rs:16` `pub fn rust_loc_files_create_sql` — the doc is only an `# Errors` section, so rustdoc's summary line for the item is the word "Errors"
- `views.rs:26` `pub fn rust_loc_summary_view_sql` — the doc is two rule-ID/task-ID rationale paragraphs (see the companion READ-13 finding) and never says the function returns the `CREATE OR REPLACE VIEW` statement for `rust_loc_summary`
- `counter.rs:108` `pub const fn Locs::lines` — the doc is a saturating-arithmetic proof; the summary line reads as a proof, not as "total lines across the four buckets"

**Why it matters**: Consistent with the same finding filed against ops-duckdb (TASK-2130), ops-about (TASK-2071), ops-extension (TASK-2098), ops-tokei (TASK-2164), ops-cargo-toml (TASK-2145) and cargo-update (TASK-2152). `cargo doc` for this crate shows a list of blank entries next to the well-documented ones, and the `# Errors`-only case actively misrenders in the item index.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every public item listed above has a one-line doc summary stating what it is, in the sentence form used elsewhere in the crate
- [ ] #2 rust_loc_files_create_sql leads with a summary sentence before its # Errors section
- [ ] #3 rust_loc_summary_view_sql and Locs::lines lead with a summary sentence; any retained rationale follows it
- [ ] #4 cargo doc -p ops-rust-loc produces no missing-docs style gaps for the listed items and still builds without warnings
<!-- AC:END -->
