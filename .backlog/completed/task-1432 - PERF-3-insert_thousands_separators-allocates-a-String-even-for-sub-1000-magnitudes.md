---
id: TASK-1432
title: >-
  PERF-3: insert_thousands_separators allocates a String even for sub-1000
  magnitudes
status: Done
assignee:
  - TASK-1458
created_date: '2026-05-13 18:23'
updated_date: '2026-05-14 08:25'
labels:
  - code-review-rust
  - PERF
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/text.rs:166`

**What**: The early-return path `if len <= 3 { return digits.to_string(); }` still allocates for every input under 1000.

**Why it matters**: `format_number` is called per row in language breakdowns and about-card metrics; small magnitudes dominate. Returning `Cow<'_, str>` (borrowed for the no-separator case, owned for the inserted case) lets callers skip the allocation entirely.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Return Cow<'_, str> from insert_thousands_separators (or equivalent zero-alloc fast path)
- [ ] #2 Call sites adapted; no string copy on inputs with len <= 3
<!-- AC:END -->
