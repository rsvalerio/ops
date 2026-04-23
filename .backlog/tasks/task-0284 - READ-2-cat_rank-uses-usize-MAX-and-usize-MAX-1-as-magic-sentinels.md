---
id: TASK-0284
title: 'READ-2: cat_rank uses usize::MAX and usize::MAX-1 as magic sentinels'
status: To Do
assignee: []
created_date: '2026-04-23 06:37'
updated_date: '2026-04-23 06:46'
labels:
  - rust-code-review
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/help.rs:99`

**What**: Comparator mixes three rank classes via numeric magic.

**Why it matters**: Cognitive load in a small sorting primitive.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract enum CategoryRank { Known(usize), Unknown, None }
- [ ] #2 impl Ord and simplify sort_entries_by_category
<!-- AC:END -->
