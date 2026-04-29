---
id: TASK-0588
title: 'API-9: pub struct AboutOptions lacks #[non_exhaustive]'
status: Done
assignee:
  - TASK-0636
created_date: '2026-04-29 05:18'
updated_date: '2026-04-29 06:16'
labels:
  - code-review-rust
  - API
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/Cargo.toml`
**File**: `extensions/about/src/lib.rs:64`

**What**: AboutOptions is pub with 3 flag fields (refresh, visible_fields, is_tty), the entry-point parameter for run_about. Out-of-crate callers construct via struct literal. Adding a fourth knob would break every caller.

**Why it matters**: API-9 — repo policy is non_exhaustive on extension-facing pub structs. AboutOptions will accumulate options over time.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 AboutOptions annotated #[non_exhaustive]
- [ ] #2 pub fn new(...) constructor (or builder) provided so out-of-crate construction has a stable path
- [ ] #3 Existing call sites compile unchanged
<!-- AC:END -->
