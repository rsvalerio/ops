---
id: TASK-1241
title: 'SEC-33: validate_extra_opts has no upper bound on pair count or token length'
status: Done
assignee:
  - TASK-1260
created_date: '2026-05-08 12:59'
updated_date: '2026-05-08 14:17'
labels:
  - code-review-rust
  - security
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/validation.rs:144-165`

**What**: `validate_extra_opts` enforces an alphanumeric/`_` allowlist per token but caps neither `opts.len()` nor the number of `key=value` pairs. The function is pub, documented as the safety boundary for the only fragment that gets *interpolated* into `read_json_auto(..., {opts})`, so a future dynamic caller can pass a multi-megabyte allowlist-conformant string with no validator pushback.

**Why it matters**: SEC-33 resource-exhaustion gap on the interpolated-fragment surface. Today's call sites are static literals, but the safety contract is documented at this validator and a missing cap silently widens it for future callers.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Add a hard cap on opts.len() (e.g. 4 KiB) and pair count (e.g. 32)
- [x] #2 Tests for oversize input pinning the cap
- [x] #3 Document the cap in the function-level doc as part of the safety contract
<!-- AC:END -->
