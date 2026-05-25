---
id: TASK-1339
title: >-
  TEST-12: dry_run_expands_composite_commands and
  dry_run_composite_shows_all_steps are near-duplicate tests
status: Done
assignee:
  - TASK-1387
created_date: '2026-05-12 16:27'
updated_date: '2026-05-13 07:59'
labels:
  - code-review-rust
  - tests
  - duplication
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/run_cmd/tests.rs:473-492` and `:625-637`

**What**: Both tests build the same `build_test_runner()`, call `run_command_dry_run_to(&runner, "verify", &mut buf)`, and assert the same three substrings (`"Resolved to 2 step(s)"`, `"[1] build"`, `"[2] test"`). The second adds two extra assertions but otherwise verifies the same expansion path.

**Why it matters**: Two tests pinning the same behaviour add maintenance cost (any output tweak updates both) without raising coverage. Either collapse or repurpose one to a distinct scenario (nested/parallel composite).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either the two tests are collapsed into one (with the union of assertions), or one is retitled and rebuilt to cover a distinct composite scenario.
- [ ] #2 No regression in coverage for composite dry-run rendering.
<!-- AC:END -->
