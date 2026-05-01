---
id: TASK-0797
title: >-
  DUP-2: read_i64_field and read_f64_field are structurally identical, differ
  only in the JSON accessor and default type
status: Done
assignee:
  - TASK-0822
created_date: '2026-05-01 06:00'
updated_date: '2026-05-01 06:57'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/lib.rs:143-171`

**What**: Both functions match on section.get(field), return the typed default on None, and call as_i64() / as_f64() with a tracing::warn fallback on shape mismatch. Bodies are 13 lines each; only the typed accessor + fallback literal change.

**Why it matters**: A future schema-drift policy change has to edit two functions in lockstep — DUP-2 anti-pattern.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract a single helper parameterised over the JSON accessor closure and default value (e.g. read_field with closure |v| v.as_i64())
- [ ] #2 read_i64_field / read_f64_field either disappear or become one-liners delegating to the helper
- [ ] #3 Behavior unchanged: schema-drift still emits one tracing::warn with the same fields
<!-- AC:END -->
