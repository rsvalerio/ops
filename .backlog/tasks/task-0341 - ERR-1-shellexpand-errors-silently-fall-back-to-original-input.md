---
id: TASK-0341
title: 'ERR-1: shellexpand errors silently fall back to original input'
status: Done
assignee:
  - TASK-0420
created_date: '2026-04-26 09:34'
updated_date: '2026-04-27 11:25'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/expand.rs:65`

**What**: shellexpand::full_with_context(...).unwrap_or(Cow::Borrowed(input)) swallows every error variant — not just unset variable. A real VarError::NotUnicode from the env lookup is mapped to "pass through unchanged" with no log.

**Why it matters**: The doc-comment contract only describes the unresolved-var path; conflating it with genuine errors hides config bugs (e.g. a non-UTF-8 env value) and violates ERR-1.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Match on the error and only fall back on the unresolved-variable case; log other errors at warn/debug with the offending variable name
- [ ] #2 Add a test that injects a deliberately failing lookup to lock the new contract in
<!-- AC:END -->
