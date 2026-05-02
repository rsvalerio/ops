---
id: TASK-0835
title: 'ERR-7: ExpandError::cause collapses underlying error chain to a single String'
status: Triage
assignee: []
created_date: '2026-05-02 09:12'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/expand.rs:19-35,128-132`

**What**: ExpandError stores cause: String produced via err.cause.to_string(), so the original shellexpand/std::env::VarError source is discarded. There is no Error::source() impl that walks back to the underlying VarError.

**Why it matters**: ERR-9 calls for an Error::source() chain so callers and tracing formatters can render the full cause; downgrading to a String defeats {:#} and structured-error tooling and runs counter to the SharedError pattern the extension crate uses (error.rs:9).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace cause: String with cause: shellexpand::LookupError<std::env::VarError> (or Box<dyn Error + Send + Sync>)
- [ ] #2 Implement Error::source() returning the wrapped error
- [ ] #3 Adjust the Display impl to format only the immediate message; keep tests pinning var_name and to_string() shape
<!-- AC:END -->
