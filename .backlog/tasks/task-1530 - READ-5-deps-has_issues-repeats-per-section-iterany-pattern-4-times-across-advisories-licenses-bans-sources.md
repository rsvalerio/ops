---
id: TASK-1530
title: >-
  READ-5: deps has_issues repeats per-section iter+any pattern 4 times across
  advisories/licenses/bans/sources
status: To Do
assignee:
  - TASK-1646
created_date: '2026-05-19 07:33'
updated_date: '2026-05-25 16:08'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/lib.rs:230-275`

**What**: The four chained `||` clauses over `advisories` / `licenses` / `bans` / `sources` differ only in the slice and the `relax_warning` flag. A small table or helper would shorten the body and make it obvious that bans is the only `relax_warning = true` case.

**Why it matters**: The current layout makes a future "advisories also relax warnings" or "add a new section" change a four-arm copy-paste that's easy to get wrong; one of the four was already special-cased differently.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Introduce a local [(slice, relax)] array and a single .iter().any(...) over it (or a per-section helper)
- [ ] #2 Bans relaxation stays a single boolean
- [ ] #3 Existing has_issues tests pass
<!-- AC:END -->
