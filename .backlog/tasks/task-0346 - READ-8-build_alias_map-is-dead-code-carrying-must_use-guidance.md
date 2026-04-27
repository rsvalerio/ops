---
id: TASK-0346
title: 'READ-8: build_alias_map is dead code carrying #[must_use] guidance'
status: To Do
assignee:
  - TASK-0421
created_date: '2026-04-26 09:34'
updated_date: '2026-04-26 10:10'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/mod.rs:84-98`

**What**: Config::build_alias_map is declared pub, marked #[must_use], and documented as the O(1)-lookup answer to resolve_alias's O(N*M) scan, but no caller in the workspace uses it.

**Why it matters**: Keeping unused public API muddies the surface (CL-3/READ-1).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Remove build_alias_map (and update resolve_alias doc) if no consumer exists
- [ ] #2 Or add at least one caller in run_cmd/runner that uses the cached map for repeated alias lookups; cover with a test
<!-- AC:END -->
