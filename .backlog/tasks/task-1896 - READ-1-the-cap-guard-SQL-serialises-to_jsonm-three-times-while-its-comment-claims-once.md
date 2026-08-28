---
id: TASK-1896
title: >-
  READ-1: the cap-guard SQL serialises to_json(m) three times while its comment
  claims once
status: To Do
assignee:
  - TASK-1999
created_date: '2026-08-27 15:36'
updated_date: '2026-08-28 14:13'
labels:
  - code-review-rust
  - structure-readability
dependencies: []
modified_files:
  - extensions-rust/metadata/src/lib.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/lib.rs:263-283`

<!-- scan confidence: candidates to inspect -->

**What**: The comment above the query states the fix explicitly:

> "the previous shape ran `to_json(m)::VARCHAR` server-side twice (once for `octet_length`, once to fetch the text) ... The new shape uses a CASE expression: `octet_length(...)` is computed once and returned alongside the payload"

The SQL immediately below writes the serialisation out **three** times:

```sql
SELECT octet_length(CAST(to_json(m)::VARCHAR AS BLOB)) AS bytes,
       CASE WHEN octet_length(CAST(to_json(m)::VARCHAR AS BLOB)) > ?
            THEN NULL ELSE to_json(m)::VARCHAR END AS payload
FROM metadata_raw m
```

`octet_length(CAST(to_json(m)::VARCHAR AS BLOB))` appears twice and `to_json(m)::VARCHAR` a third time in the ELSE branch. Whether this actually costs three serialisations depends entirely on DuckDB performing common-subexpression elimination across the projection list and into the CASE — which the comment neither claims nor tests. If DuckDB does not CSE here, the change that was made to *halve* the cost on the common under-cap path tripled it instead; if it does, the comment is describing an optimiser behaviour as though it were a property of the SQL. Either way a reader cannot tell which, and nothing pins it.

The shape the comment describes is expressible directly and needs no optimiser assumption:

```sql
WITH j AS (SELECT to_json(m)::VARCHAR AS txt FROM metadata_raw m)
SELECT octet_length(CAST(txt AS BLOB)) AS bytes,
       CASE WHEN octet_length(CAST(txt AS BLOB)) > ? THEN NULL ELSE txt END AS payload
FROM j
```

Related observation: `query_metadata_raw_rejects_oversized_payload_before_materialising` (tests/payload_cap.rs:89) seeds a 100 MiB value and then runs this query, so if the repetition is real that test drives ~300 MiB of server-side JSON serialisation to prove a point about a 1 MiB cap.

**Why it matters**: READ-1 — the comment is the only documentation of a deliberate performance decision, and it does not match the code it sits on. A future reader optimising this path will trust the comment and look elsewhere. The correctness of the cap guard is unaffected; the cost claim is what is unverified.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The SQL evaluates to_json(m)::VARCHAR exactly once per row, or the comment is corrected to state that it relies on DuckDB CSE and cites the evidence
- [ ] #2 EXPLAIN ANALYZE (or an equivalent measurement) is recorded in the task or the comment showing the actual number of serialisations on the under-cap path
- [ ] #3 The over-cap path still returns NULL for the payload so no oversized String crosses the FFI boundary (SEC-33 behaviour preserved, existing payload_cap tests stay green)
<!-- AC:END -->
