---
id: TASK-1362
title: >-
  TEST-11: hide_irrelevant_commands_* trio is vacuous when neither 'deps' nor
  'tools' subcommand is registered
status: To Do
assignee:
  - TASK-1385
created_date: '2026-05-12 21:29'
updated_date: '2026-05-12 22:16'
labels:
  - code-review-rust
  - tests
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/args.rs:402`, `:418`, `:434`

**What**: `hide_irrelevant_commands_no_stack_hides_stack_specific`, `_matching_stack_shows`, and `_wrong_stack_hides` all iterate subcommands with an inner `if name == "deps" || name == "tools"` gate. Under a feature combo that drops both subcommands, the loops run but the assertions never fire — the tests pass with zero substantive checks.

**Why it matters**: TEST-11 vacuous assertion: a build that no longer ships `deps`/`tools` as subcommands turns the trio into no-op passes. Solution: pre-assert the expected name set is present in `result.get_subcommands()` before checking per-name visibility; or extract the expected names from `stack_specific_commands()` so future stack-specific additions are covered automatically.

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Pre-assert the expected stack-specific subcommand name set exists before iterating; tests fail loudly if neither 'deps' nor 'tools' is present
- [ ] #2 Consolidate the three sibling tests through a table or helper sourcing the expected names from stack_specific_commands()
<!-- AC:END -->
