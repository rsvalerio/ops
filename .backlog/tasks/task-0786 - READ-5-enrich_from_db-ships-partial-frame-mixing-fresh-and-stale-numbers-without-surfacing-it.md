---
id: TASK-0786
title: >-
  READ-5: enrich_from_db ships partial frame mixing fresh and stale numbers
  without surfacing it
status: Done
assignee:
  - TASK-0826
created_date: '2026-05-01 05:58'
updated_date: '2026-05-01 09:33'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/units.rs:117`

**What**: When unit.path.is_empty() || == "." and project_loc = None (query failed; warn was logged), the code path leaves unit.loc = None even though query_crate_loc/query_crate_file_count succeeded with values applicable elsewhere. The two query results are independent — the same unit can end up with loc = Some and file_count = None silently when one query fails mid-render.

**Why it matters**: TASK-0431/TASK-0463 covered the four independent queries; this is the residual hazard where partial enrichment ships a frame mixing fresh and stale numbers without surfacing it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 When any of the four queries fails, surface a single warn including which fields were left untouched (vs four scattered warns), so an operator reading logs sees the rendered frame is partial
- [ ] #2 Optional: add a follow-up flag on ProjectIdentity indicating partial enrichment
<!-- AC:END -->
