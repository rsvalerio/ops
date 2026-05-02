---
id: TASK-0862
title: 'SEC-21: external_err flattens diagnostic chain via to_string instead of {e:#}'
status: Done
assignee: []
created_date: '2026-05-02 09:20'
updated_date: '2026-05-02 10:38'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/ingest.rs:135-137`

**What**: external_err(e: impl Display) -> DbError::External(e.to_string()) calls e.to_string() which only renders the head of the error chain. Source chains attached via anyhow::Context are lost - DbError::External always shows just the leaf cause.

**Why it matters**: tokei, coverage, and any future external_err consumer will all surface a one-line cause. The {e:#} formatter (alternate Debug) preserves the chain. Operator triage is substantially harder when collect_tokei fails in a permission-denied subdirectory deep in the workspace.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Change external_err to format!("{e:#}") so source chains are preserved
- [ ] #2 Add a unit test that wraps an anyhow::Error::msg(a).context(b).context(c) through external_err and asserts the resulting DbError::External contains all three layers
- [ ] #3 Verify no log site relies on the single-line shape (search for DbError::External consumers)
<!-- AC:END -->
