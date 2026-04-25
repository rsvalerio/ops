---
id: TASK-0308
title: >-
  DUP-1: strip_userinfo / redact_userinfo duplicated between
  extensions/git/src/provider.rs and config.rs
status: Done
assignee:
  - TASK-0325
created_date: '2026-04-24 08:53'
updated_date: '2026-04-25 12:50'
labels:
  - rust-code-review
  - duplication
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: extensions/git/src/provider.rs:68-81 and extensions/git/src/config.rs:46-59

**What**: Two near-identical URL userinfo scrubbers in sibling modules.

**Why it matters**: Drift risk — a redaction fix (SEC-21) in one won't propagate to the other.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Logic consolidated into a single pub(crate) fn in one module
- [ ] #2 Both call sites use it; single set of tests covers both callers
<!-- AC:END -->
