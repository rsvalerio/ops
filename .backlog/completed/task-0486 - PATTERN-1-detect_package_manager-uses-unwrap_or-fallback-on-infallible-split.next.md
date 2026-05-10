---
id: TASK-0486
title: >-
  PATTERN-1: detect_package_manager uses unwrap_or fallback on infallible
  split().next()
status: Done
assignee:
  - TASK-0532
created_date: '2026-04-28 06:08'
updated_date: '2026-04-28 15:44'
labels:
  - code-review-rust
  - pattern
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/package_manager.rs:12`

**What**: `let name = pm.split(\@\).next().unwrap_or(pm);` — str::split always yields at least one element, so next() is infallible and the unwrap_or branch is unreachable.\n\n**Why it matters**: Defensive fallbacks on infallible iterators mislead readers into thinking next() can be None here. Use split_once to express the real intent (strip an optional @version suffix) without a bogus fallback.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace with pm.split_once('@').map(|(n, _)| n).unwrap_or(pm) or equivalent
- [ ] #2 Behaviour unchanged for pnpm@9.0.0, pnpm, bun@1
- [ ] #3 Existing package_manager_field_takes_precedence test continues to pass
<!-- AC:END -->
