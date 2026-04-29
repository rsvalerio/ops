---
id: TASK-0632
title: >-
  READ-4: compute_module_count precedence rule lives in 11-line doc-comment but
  function is 8 lines
status: Triage
assignee: []
created_date: '2026-04-29 05:22'
labels:
  - code-review-rust
  - READ
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/lib.rs:89`

**What**: compute_module_count (lib.rs:89-108) has 11-line doc-comment explaining tri-state precedence (go.work first, go.mod second, otherwise None) plus "single-module case returns None" subtlety. Body is 7 lines. Function is small enough but doc-comment encodes a "single-module returns None" invariant enforced by the trailing .then_some(count) — easy to forget in maintenance.

**Why it matters**: When prose-to-code ratio is >1, prose tends to drift. Encoding the rule as a named local makes the invariant self-documenting.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 compute_module_count body uses named binding for 'extras present' predicate
- [ ] #2 Doc-comment shrinks to describe only precedence rule
- [ ] #3 Tests pin both workspace-precedence and single-module-returns-None branches
<!-- AC:END -->
