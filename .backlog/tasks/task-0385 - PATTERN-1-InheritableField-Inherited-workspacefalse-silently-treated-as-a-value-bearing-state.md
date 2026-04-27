---
id: TASK-0385
title: >-
  PATTERN-1: InheritableField Inherited workspace=false silently treated as a
  value-bearing state
status: To Do
assignee:
  - TASK-0417
created_date: '2026-04-26 09:39'
updated_date: '2026-04-26 10:10'
labels:
  - code-review-rust
  - patterns
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/inheritance.rs:69`

**What**: resolve_string_field only acts when the field is Inherited { workspace: true }. The workspace: false case is parseable from TOML but never resolved nor reported as an error — it becomes an empty/default field. Cargo itself rejects workspace = false in this position.

**Why it matters**: Differs from cargo behavior; users who write workspace = false get a silently misparsed manifest with empty fields.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either reject workspace = false during parse with a domain error or document the chosen permissive behavior
- [ ] #2 Test asserting the chosen behavior
<!-- AC:END -->
