---
id: TASK-0855
title: >-
  ERR-7: query_or_warn collapses MutexPoisoned/Timeout to render-time fallback
  at warn level
status: Done
assignee: []
created_date: '2026-05-02 09:18'
updated_date: '2026-05-02 14:29'
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
- [x] #1 Match on DbError::MutexPoisoned (and DbError::Timeout) inside query_or_warn and log at error! (not warn!), or propagate them up rather than collapsing to fallback
- [x] #2 Add a test exercising query_or_warn with DbError::MutexPoisoned and asserting the elevated severity
- [x] #3 Document the contract in the rustdoc
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
query_or_warn now classifies failures via is_hard_failure, which walks the anyhow chain (so .context()-wrapped errors still detect): DbError::MutexPoisoned and DbError::Timeout escalate to tracing::error! while everything else stays at warn!. Module rustdoc documents the contract. Added 5 tests pinning the classification, the anyhow-context walk, and the fallback-still-returns behaviour. Note: pre-existing unrelated test-compile failure in ops-duckdb (lib.rs:184 uses Config::default that no longer exists) prevents `cargo test -p ops-duckdb`; ops verify (build/clippy) is clean.
<!-- SECTION:NOTES:END -->
