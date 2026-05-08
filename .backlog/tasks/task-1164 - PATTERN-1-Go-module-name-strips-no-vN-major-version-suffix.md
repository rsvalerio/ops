---
id: TASK-1164
title: 'PATTERN-1: Go module name strips no /vN major-version suffix'
status: To Do
assignee:
  - TASK-1270
created_date: '2026-05-08 07:45'
updated_date: '2026-05-08 13:19'
labels:
  - code-review-rust
  - PATTERN
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/lib.rs:65-68`

**What**: `name = go_mod.module.rsplit('/').next()` takes the literal last `/`-segment. Per Go module semantics, paths with major-version >= 2 carry a `/vN` suffix (`github.com/foo/bar/v2`, `github.com/openbao/openbao/api/v2`), so the rendered About-card name becomes `\"v2\"` instead of `\"bar\"` / `\"api\"`. Same bug in `last_segment` in `modules.rs:123-127` (used to label workspace units).

**Why it matters**: User-visible identity is wrong on a large class of real-world Go modules (kubernetes, openbao, anything past v2). Names collapse to `v2`/`v3`, making the About card and unit listing useless for the very projects that motivate workspace support.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 When the last segment matches ^v\\d+$ and a previous segment exists, drop the version segment and use the preceding segment as the name
- [ ] #2 last_segment("github.com/foo/bar/v2") returns "bar"; last_segment("github.com/openbao/openbao/api/v2") returns "api"
- [ ] #3 A bare "module v2" (no /) is left unchanged
- [ ] #4 Tests cover /v2, /v10, no-suffix, single-segment, and a path ending in /v (not a version)
<!-- AC:END -->
