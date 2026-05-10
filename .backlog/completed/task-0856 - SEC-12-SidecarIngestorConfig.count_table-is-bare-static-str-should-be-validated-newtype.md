---
id: TASK-0856
title: >-
  SEC-12: SidecarIngestorConfig.count_table is bare static str; should be
  validated newtype
status: Done
assignee: []
created_date: '2026-05-02 09:18'
updated_date: '2026-05-02 14:31'
labels:
  - code-review-rust
  - security
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/ingestor.rs:172-186`

**What**: count_records_with builds format SELECT COUNT(*) FROM quoted from quoted produced by quoted_ident(self.count_table) at line 131. count_table is static str today, but SidecarIngestorConfig is pub and non_exhaustive with a pub const fn new constructor - out-of-crate consumers can pass any static str literal. The existing test catches injection at runtime.

**Why it matters**: The static str bound merely says the byte buffer outlives the program, not this string was hand-written. Hardening the field to a validated newtype (mirroring TableName in helpers.rs) would make wrong values a compile error rather than a runtime SqlValidation Err.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Replace the bare static str for count_table with a TableName-style newtype constructed via a fallible const fn or builder
- [x] #2 Construction-site call sites compile unchanged
- [x] #3 Removal of the runtime quoted_ident(self.count_table) in load_with_sidecar becomes safe
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Added TableName newtype in extensions/duckdb/src/sql/validation.rs with const-time validation via const fn is_valid_identifier_const + assert!. SidecarIngestorConfig.count_table is now a TableName; SidecarIngestorConfig::new stays const fn and accepts &static str (validates via TableName::from_static at compile time). Construction sites that took raw literals now compile unchanged through ::new; struct-init test sites updated to use TableName::from_static. load_with_sidecar dropped the runtime quoted_ident call (uses self.count_table.quoted() — pre-validated). The previous load_with_sidecar_returns_error_for_invalid_count_table runtime test was replaced with a const-context test that exercises the validator from a const initializer (so a future loosening trips a build failure).
<!-- SECTION:NOTES:END -->
