---
id: TASK-0599
title: 'ERR-1: query_metadata_raw uses LIMIT 1 without enforcing single-row invariant'
status: Triage
assignee: []
created_date: '2026-04-29 05:19'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/lib.rs:165`

**What**: metadata_raw is loaded with record_count = 1u64 and queried as `SELECT to_json(m) FROM metadata_raw LIMIT 1`. If a future ingest path inserts more than one row (re-collect without truncate, schema-version row), the provider silently picks "some" row. Invariant lives only in a comment in MetadataIngestor::load.

**Why it matters**: Encoding singleton-table invariant in LIMIT 1 is fragile — bug surfaces as wrong workspace data with no diagnostic.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Read-side asserts row count == 1 and warns/errors otherwise, OR schema enforces single-row via PRIMARY KEY
- [ ] #2 Test covers multi-row regression path
<!-- AC:END -->
