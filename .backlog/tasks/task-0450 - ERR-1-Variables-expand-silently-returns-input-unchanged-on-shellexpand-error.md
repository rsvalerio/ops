---
id: TASK-0450
title: 'ERR-1: Variables::expand silently returns input unchanged on shellexpand error'
status: Done
assignee:
  - TASK-0537
created_date: '2026-04-28 05:44'
updated_date: '2026-04-28 16:44'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/expand.rs:65-71`

**What**: When `shellexpand::full_with_context` errors (e.g. `VarError::NotUnicode`), `expand` logs a warning and returns `Cow::Borrowed(input)`. The literal `${VAR}` flows through as-is into the resolved command argv.

**Why it matters**: Downstream code treating the result as a path will create directories named `${OPS_DATA_DIR}` on disk. An env-config bug downgrades to a tracing warning while the wrong path is materialized in the user filesystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Make expand return Result (or have callers opt into a strict mode) so a non-UTF-8 env var fails loudly
- [x] #2 Tests verify the failure path exits with a user-visible error rather than leaving ${VAR} literal in the command
<!-- AC:END -->
