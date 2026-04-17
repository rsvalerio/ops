---
id: TASK-0090
title: 'FN-1: run_about orchestrates too many concerns (~46 lines, 6 branches)'
status: Done
assignee: []
created_date: '2026-04-17 11:33'
updated_date: '2026-04-17 15:48'
labels:
  - rust-codereview
  - fn
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/lib.rs:53`

**What**: run_about sets up context, pre-warms 3 providers, handles refresh-specific coverage error, dispatches identity resolution, enriches from DB, and renders — all inline.

**Why it matters**: Close to FN-1 limit and hard to test; currently untestable because it uses std::env::current_dir() and println!.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract pre-warm providers into warm_generic_providers(&mut ctx, registry, refresh)
- [ ] #2 Extract resolve_identity(ctx, registry, cwd) returning ProjectIdentity
- [ ] #3 Accept cwd and writer as parameters to make function pure and testable
<!-- AC:END -->
