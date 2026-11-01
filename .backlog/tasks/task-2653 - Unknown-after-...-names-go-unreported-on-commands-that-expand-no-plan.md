---
id: TASK-2653
title: 'Unknown after = [...] names go unreported on commands that expand no plan'
status: Done
assignee: []
created_date: '2026-11-01 00:45'
updated_date: '2026-11-01 13:32'
labels:
  - code-review-rust
  - config
dependencies: []
parent_task_id: 'TASK-2656'
modified_files:
  - crates/runner/src/command/resolve.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/resolve.rs`

**What**: Unknown names in a step's `after` list are rejected by `expand_to_plan`, not `Config::validate`, because extensions register after config load. Commands that never expand a plan, such as `ops about`, therefore do not report a bad name.

**Why it matters**: TASK-2637 AC #4 asked for a load error. The substitution covers `ops <cmd>`, `--dry-run` and `ops explain`, but a typo stays silent until one of those runs.

**Origin**: discovered during TASK-2644 while fixing TASK-2637; surfaced in the run-level report.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Either plan-less commands report an unknown after name, or the limit is documented as a decision

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Resolved via the AC's second branch (documented decision): unknown `after` names are checked in `expand_to_plan` because only the plan-expansion path has the extension and builtin stores the check resolves against; `ops about` and friends never build them, and a construction-time check would falsely reject names that a registered extension defines (the runner constructor is also deliberately infallible). docs/configuration.md's 'Ordering steps with after' section now states the limit as a deliberate decision, and `check_after_names`'s doc comment in crates/runner/src/command/resolve.rs records the rationale at the code site. No behaviour change.
<!-- SECTION:NOTES:END -->
