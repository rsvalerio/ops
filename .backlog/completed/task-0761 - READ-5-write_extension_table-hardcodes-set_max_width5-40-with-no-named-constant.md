---
id: TASK-0761
title: >-
  READ-5: write_extension_table hardcodes set_max_width(5, 40) with no named
  constant
status: Done
assignee:
  - TASK-0828
created_date: '2026-05-01 05:54'
updated_date: '2026-05-02 08:02'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/extension_cmd.rs:87`

**What**: `table.set_max_width(5, 40);` — column index 5 is description column and 40 is truncation width. Both magic numbers. Header constructed two lines earlier; if a column is added or reordered, the 5 silently truncates the wrong column. Symmetric one-liner in print_provider_info (set_max_width(2, 50)).

**Why it matters**: Coupling the index and header order via positional integers is a classic READ-5 invariant-not-explicit smell.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Description column index is computed from headers vector (e.g. headers.iter().position(|h| *h == "Description")) or held in a named constant DESCRIPTION_COL: usize = 5
- [ ] #2 Truncation width 40 lives in a named constant or is justified inline
- [ ] #3 Same treatment applied to print_provider_info for symmetry
<!-- AC:END -->
