---
id: TASK-1392
title: >-
  READ-1: compose_stack_value and compose_project_value duplicate non-empty-push
  idiom four times across two near-identical composers
status: Done
assignee:
  - TASK-1452
created_date: '2026-05-13 18:03'
updated_date: '2026-05-13 20:35'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/project_identity/format.rs` (compose_stack_value and compose_project_value)

**What**: Both composers repeat the `if !s.is_empty() { parts.push(s.clone()) }` and `if let Some(s) = &opt { if !s.is_empty() { parts.push(s.clone()) } }` idiom four times across their bodies, then call `parts.join("\n")` and return `None` on empty. The two functions are structurally identical apart from which `ProjectIdentity` fields they read.

**Why it matters**: READ-1 / DUP — every fix to the non-empty semantics (trimming whitespace, treating "n/a" as empty, etc.) has to land in four spots inside two functions, and the divergence is invisible at a glance.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract two helpers, push_non_empty(parts: &mut Vec<String>, s: &str) and push_non_empty_opt(parts: &mut Vec<String>, opt: Option<&str>), and replace the four open-coded idioms across compose_stack_value and compose_project_value
- [ ] #2 Keep public signatures and return values of the composers unchanged; the existing tests covering empty and partial inputs must continue to pass without modification
<!-- AC:END -->
