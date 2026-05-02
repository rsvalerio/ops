---
id: TASK-0856
title: >-
  SEC-12: SidecarIngestorConfig.count_table is bare static str; should be
  validated newtype
status: Triage
assignee: []
created_date: '2026-05-02 09:18'
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
- [ ] #1 Replace the bare static str for count_table with a TableName-style newtype constructed via a fallible const fn or builder
- [ ] #2 Construction-site call sites compile unchanged
- [ ] #3 Removal of the runtime quoted_ident(self.count_table) in load_with_sidecar becomes safe
<!-- AC:END -->
