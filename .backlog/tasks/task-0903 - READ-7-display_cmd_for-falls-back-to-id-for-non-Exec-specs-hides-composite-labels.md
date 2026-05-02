---
id: TASK-0903
title: >-
  READ-7: display_cmd_for falls back to id for non-Exec specs, hides composite
  labels
status: Triage
assignee: []
created_date: '2026-05-02 10:09'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/run_cmd/plan.rs:34`

**What**: display_cmd_for(runner, id) returns the bare id for any CommandSpec variant other than Exec — composites and any future variant render in the progress UI as the raw id rather than a human-friendly label. The wildcard `_ => id.to_string()` defeats exhaustiveness on CommandSpec.

**Why it matters**: Plan display rows for composite leaves show internal ids instead of resolved labels; future CommandSpec variants regress silently rather than failing to compile.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Match on CommandSpec exhaustively (composite + any new variant) and produce a meaningful label per variant
- [ ] #2 Replace the catch-all arm with explicit arms
- [ ] #3 Test that asserts display_cmd_for returns a non-id label for Composite specs
<!-- AC:END -->
