---
id: TASK-0868
title: >-
  CL-3: terraform find_required_version candidate list contains a '.' sentinel
  that is continued on
status: Done
assignee: []
created_date: '2026-05-02 09:21'
updated_date: '2026-05-02 10:47'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/about/src/lib.rs:69-75`

**What**: let candidates = [".", "versions.tf", ...] - the loop body checks if candidate == "." { continue; }. The "." element is dead weight that exists only to be skipped.

**Why it matters**: The reader has to track why the first element is skipped; deletion is a strict simplification with no behavioral change.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Remove . from the array and the continue guard
- [ ] #2 Confirm tests still pass unchanged
<!-- AC:END -->
