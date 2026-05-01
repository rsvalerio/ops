---
id: TASK-0753
title: >-
  PATTERN-3: CommandRegistry::insert clones CommandId on every duplicate
  registration
status: Triage
assignee: []
created_date: '2026-05-01 05:53'
labels:
  - code-review-rust
  - patterns
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/extension.rs:125-130`

**What**: On collision, insert calls self.duplicate_inserts.push(id.clone()) then self.inner.insert(id, spec). CommandId wraps String, so the clone allocates a fresh heap buffer for the audit trail every time. The contains-key check + clone could be elided by inserting first and reading the prior key from the map on Some(_).

**Why it matters**: PATTERN-3 / OWN-8: avoid allocations in audit paths. Pair with TASK-0653 (Clone) and TASK-0652 (DerefMut) clean-ups.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Refactor insert to call inner.insert first and detect collision via the returned Option<CommandSpec>, pushing the key only when needed and without cloning the input
- [ ] #2 take_duplicate_inserts audit semantics preserved by existing tests
<!-- AC:END -->
