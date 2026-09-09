---
id: TASK-2194
title: 'TEST-11: the 100-MiB cap-ordering test asserts only the error text, which the defect it pins produced identically'
status: Done
assignee: []
created_date: '2026-09-08 07:14'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - tests
dependencies: []
parent_task_id: 'TASK-2240'
modified_files:
  - extensions-rust/metadata/src/tests/payload_cap.rs
priority: medium
ordinal: 107000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/tests/payload_cap.rs:88`

**What**: `query_metadata_raw_rejects_oversized_payload_before_materialising`
seeds a 100 MiB `metadata_raw` row and calls
`query_metadata_raw_with_cap(&db, 1 MiB)`. Its doc comment states the purpose
plainly: pin that the cap fires **before** the payload is materialised into a
Rust `String`, the defect fixed by SEC-33 / TASK-1194.

The two assertions are:

```rust
assert!(msg.contains("exceeds") && msg.contains("byte cap"), ...);
assert!(msg.contains(METADATA_MAX_BYTES_ENV), ...);
```

Both are properties of the error *message*, and the pre-TASK-1194
materialise-then-check implementation produced exactly that message — it
allocated the 100 MiB `String` first and then bailed with the same text.
Nothing in the test observes allocation, peak RSS, the `CASE … THEN NULL`
shape of `CAP_GUARD_SQL`, or the query plan. Reverting to the old ordering
leaves this test green.

It is also the crate's most expensive test: a 100 MiB DuckDB value is
materialised on every `cargo test` run to assert two substrings that the
adjacent `query_metadata_raw_errors_when_payload_exceeds_cap` (32-byte cap,
one row) already asserts.

**Why it matters**: the test costs 100 MiB of RSS and real wall-clock on every
run while certifying nothing the cheap sibling does not. A reviewer reading the
doc comment reasonably believes the ordering is pinned; it is not, so the
SEC-33 guarantee can regress silently. Either assert something that
distinguishes the two implementations — e.g. the `CAP_GUARD_SQL` `payload`
column is `NULL` over cap, or an `EXPLAIN`-shape assertion in the style of the
existing `cap_guard_sql_serialises_to_json_once` — or drop the 100 MiB fixture
and fold the message assertions into the cheap test.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The test asserts a property that the pre-TASK-1194 materialise-then-check implementation would fail (e.g. the cap-guard SQL returns NULL for the payload when over cap, or a plan/shape assertion), not only the rendered error text
- [ ] #2 The 100 MiB DuckDB fixture is either justified by the new assertion or removed in favour of the existing small-cap test
- [ ] #3 The doc comment's claim matches what the assertions actually check
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Landed in wave TASK-2240. Replaced the 100-MiB fixture with cap_guard_sql_nulls_the_payload_over_cap_before_it_crosses_ffi: small two-row fixture, asserts the CASE guard NULLs the over-cap payload at the FFI boundary and passes the under-cap payload through — the property the pre-TASK-1194 materialise-then-check shape cannot produce. Error-text assertions remain in the cheap sibling test; doc comment rewritten to match (AC3).
<!-- SECTION:NOTES:END -->
