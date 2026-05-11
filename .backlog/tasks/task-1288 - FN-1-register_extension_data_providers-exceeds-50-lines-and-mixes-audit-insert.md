---
id: TASK-1288
title: >-
  FN-1: register_extension_data_providers exceeds 50 lines and mixes audit +
  insert
status: To Do
assignee:
  - TASK-1304
created_date: '2026-05-11 15:27'
updated_date: '2026-05-11 16:48'
labels:
  - code-review-rust
  - function
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/registry/registration.rs:149-213`

**What**: `register_extension_data_providers` is ~65 lines and interleaves three abstraction levels: snapshot seeding, scratch-registry construction with duplicate drain, and per-entry owner classification with four match arms emitting warnings. The symmetric `register_extension_commands` (lines 83-132) is at the boundary of FN-1 as well.

**Why it matters**: The function's correctness hinges on subtle policy (first-write-wins, four collision cases), and at 65 lines the policy logic is hard to scan in isolation. Extracting a `classify_and_warn_collision(...)` helper would isolate the warn-emission policy from the loop scaffolding.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract the per-entry owner-classification match (lines 184-211) into a helper named after its responsibility (e.g. classify_collision_first_wins)
- [ ] #2 The outer register_extension_data_providers body drops below 50 lines
- [ ] #3 All seven existing tests covering this path continue to pass
<!-- AC:END -->
