---
id: TASK-0212
title: >-
  READ-2: has_high_entropy uses magic thresholds (15/3/3/3) without named
  constants
status: Done
assignee: []
created_date: '2026-04-23 06:32'
updated_date: '2026-04-23 15:19'
labels:
  - rust-code-review
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/exec.rs:220`

**What**: The predicate inlines four magic numbers with no doc on their origin.

**Why it matters**: Hard-coded thresholds make the secret-detection heuristic hard to tune or test; readers cannot distinguish policy from typo.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Lift the four thresholds to named const with short rationale comments
- [ ] #2 Add a unit test covering a value just below each threshold
<!-- AC:END -->
