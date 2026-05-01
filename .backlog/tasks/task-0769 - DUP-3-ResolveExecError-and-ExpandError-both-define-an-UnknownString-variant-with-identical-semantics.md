---
id: TASK-0769
title: >-
  DUP-3: ResolveExecError and ExpandError both define an Unknown(String) variant
  with identical semantics
status: Triage
assignee: []
created_date: '2026-05-01 05:55'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/mod.rs:50-72`

**What**: ResolveExecError::Unknown(String) and ExpandError::Unknown(String) both mean "id not found in any store" with the same #[error("unknown command: {0}")] message. Callers in resolve.rs construct one or the other depending on the surrounding function rather than the underlying failure.

**Why it matters**: Two enums diverging on a shared concept invites drift. A single UnknownCommand(String) shared error or a From<ResolveExecError> for ExpandError impl would centralise the meaning. Both enums are #[non_exhaustive] already, refactoring is non-breaking.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either factor a shared UnknownCommand(String) into a common error or implement From<ResolveExecError> for ExpandError so the conversion is mechanical at call sites
- [ ] #2 Keep public Display strings stable so user-visible diagnostics don't change
- [ ] #3 Update tests asserting on the variants to use the unified shape
<!-- AC:END -->
