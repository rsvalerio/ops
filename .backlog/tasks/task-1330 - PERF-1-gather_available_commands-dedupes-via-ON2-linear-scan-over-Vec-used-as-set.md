---
id: TASK-1330
title: >-
  PERF-1: gather_available_commands dedupes via O(N^2) linear scan over Vec used
  as set
status: To Do
assignee:
  - TASK-1385
created_date: '2026-05-12 16:26'
updated_date: '2026-05-12 22:16'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/hook_shared.rs:150-156`

**What**: `gather_available_commands` builds `options: Vec<SelectOption>` and dedupes new entries with `options.iter().any(|o| o.name == name)` on every push, giving O(N^2) cost in command count. The in-source doc-comment dismisses this as fine for "a handful of commands," but extension + stack + config sources can extend the list without an enforced bound.

**Why it matters**: Each hook prompt re-runs the scan; with large `.ops.toml` files or extension-rich stacks the cost is unbounded. The fix is a single auxiliary `HashSet<String>` of seen names alongside the Vec.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Membership check during option construction is O(1) (HashSet-backed).
- [ ] #2 Existing hook selection tests pass; option ordering preserved.
<!-- AC:END -->
