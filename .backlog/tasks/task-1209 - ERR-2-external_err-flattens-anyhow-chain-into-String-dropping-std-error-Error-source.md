---
id: TASK-1209
title: >-
  ERR-2: external_err flattens anyhow chain into String, dropping
  std::error::Error::source
status: To Do
assignee:
  - TASK-1267
created_date: '2026-05-08 08:19'
updated_date: '2026-05-08 13:19'
labels:
  - code-review-rust
  - err
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/ingest.rs:160-162`

**What**: external_err formats its input with {e:#} and constructs DbError::External(String) — the resulting error has no #[source] chain because DbError::External(String)'s source() returns None. SEC-21 / TASK-0862 fixed the *display* of the chain but downstream consumers that walk e.source() see a single opaque string instead of the actual cause graph.

**Why it matters**: Any consumer that pattern-matches on a downcast of the leaf cause to surface a typed retry decision is permanently blind on this path.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 DbError::External is reshaped to carry #[source] anyhow::Error (or Box<dyn Error + Send + Sync>); external_err returns the wrapped error directly without the format! flattening. Display continues to render the alternate-format chain.
- [ ] #2 A unit test asserts Error::source() traversal recovers the wrapped leaf cause from a chained anyhow::Context input.
<!-- AC:END -->
