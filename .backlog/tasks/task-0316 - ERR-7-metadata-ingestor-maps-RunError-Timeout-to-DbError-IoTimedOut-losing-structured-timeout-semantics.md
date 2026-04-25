---
id: TASK-0316
title: >-
  ERR-7: metadata ingestor maps RunError::Timeout to DbError::Io(TimedOut)
  losing structured timeout semantics
status: Done
assignee:
  - TASK-0326
created_date: '2026-04-24 08:53'
updated_date: '2026-04-25 13:15'
labels:
  - rust-code-review
  - errors
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: extensions-rust/metadata/src/ingestor.rs:21-26

**What**: Timeout is flattened into a generic io::Error variant.

**Why it matters**: Callers can no longer distinguish a cargo timeout from an IO error; retry policies and user-facing messages degrade.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Add a DbError::Timeout variant (or propagate RunError)
- [ ] #2 Call sites updated to match the new variant
<!-- AC:END -->
