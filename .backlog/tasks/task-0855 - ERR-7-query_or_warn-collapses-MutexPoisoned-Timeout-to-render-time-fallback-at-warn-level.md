---
id: TASK-0855
title: >-
  ERR-7: query_or_warn collapses MutexPoisoned/Timeout to render-time fallback
  at warn level
status: Triage
assignee: []
created_date: '2026-05-02 09:18'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/mod.rs:31-42`

**What**: Every per-query failure that funnels through query_or_warn collapses to a single tracing::warn! with the error rendered via {e:#}. DbError::MutexPoisoned and DbError::Timeout are degraded identically to a transient DbError::Io - the operator-visible signal is the same. There is no facility to escalate a poisoned-mutex condition into a hard fail.

**Why it matters**: connection.rs explicitly chose MutexPoisoned policy because "a poisoned DuckDB connection reflects partially applied state we cannot trust". query_or_warn defeats that intent at the call site by presenting the failure as a render-time fallback. The coverage_color cards may keep rendering 0% over a real poisoning condition for the entire process lifetime.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Match on DbError::MutexPoisoned (and DbError::Timeout) inside query_or_warn and log at error! (not warn!), or propagate them up rather than collapsing to fallback
- [ ] #2 Add a test exercising query_or_warn with DbError::MutexPoisoned and asserting the elevated severity
- [ ] #3 Document the contract in the rustdoc
<!-- AC:END -->
