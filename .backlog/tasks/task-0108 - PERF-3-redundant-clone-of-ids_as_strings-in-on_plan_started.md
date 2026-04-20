---
id: TASK-0108
title: 'PERF-3: redundant clone of ids_as_strings in on_plan_started'
status: Done
assignee: []
created_date: '2026-04-19 18:36'
updated_date: '2026-04-19 20:28'
labels:
  - rust-code-review
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/display.rs:328`

**What**: `self.plan_command_ids = ids_as_strings.clone();` clones a `Vec<String>` that was just built on line 321 and is not used again after this assignment, so the original can be moved instead of cloned.

**Why it matters**: An avoidable per-plan-start allocation and deep string clone; violates PERF-3 (no-clone-in-hot/obvious-paths) and `clippy::redundant_clone` in spirit.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Move ids_as_strings into self.plan_command_ids without cloning
- [ ] #2 No behavioral change; cargo clippy --all-targets -- -D warnings still passes
<!-- AC:END -->
