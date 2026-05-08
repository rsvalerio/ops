---
id: TASK-1214
title: >-
  ERR-5: DuckDb::lock returns DbError::MutexPoisoned(String) capturing the full
  PoisonError display
status: To Do
assignee:
  - TASK-1268
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
**File**: `extensions/duckdb/src/connection.rs:105-112`

**What**: DuckDb::lock maps PoisonError via e.to_string() into DbError::MutexPoisoned(String). PoisonError's Display includes the mutex's panic message — which can carry the full backtrace text or a panic payload string from arbitrary user-supplied callbacks. The string is then emitted as the Display body of DbError, which flows to logs, JSON error responses, and anyhow .context() chains.

**Why it matters**: A panic message containing newlines, ANSI escapes, or operator-controlled bytes is forwarded verbatim. The captured message is unsanitised — small log-injection / error-payload-tampering vector.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 DbError::MutexPoisoned either carries the original PoisonError via #[source] and renders a fixed Display message, OR Debug-formats the captured string so embedded control bytes are escaped.
- [ ] #2 A new test simulates a panic in a closure holding the connection lock with a payload containing newline + ANSI and asserts the resulting DbError::MutexPoisoned Display contains neither raw '\n' nor '\u{1b}'.
<!-- AC:END -->
