---
id: TASK-0320
title: >-
  FN-1: flatten_coverage_json is 59 lines and repeats a 4-section
  get/and_then/unwrap_or skeleton
status: Done
assignee:
  - TASK-0326
created_date: '2026-04-24 08:54'
updated_date: '2026-04-25 13:18'
labels:
  - rust-code-review
  - complexity
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: extensions-rust/test-coverage/src/lib.rs:116-175

**What**: One function extracts 4 summary subobjects via repeated nullable-path traversal and assembles a 15-field json! literal.

**Why it matters**: Over FN-1 threshold, internally duplicative (DUP-1); adding a new subsection means repeating the pattern again.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract extract_section helper returning a typed struct
- [ ] #2 Compose the final json! literal once from the typed values
<!-- AC:END -->
