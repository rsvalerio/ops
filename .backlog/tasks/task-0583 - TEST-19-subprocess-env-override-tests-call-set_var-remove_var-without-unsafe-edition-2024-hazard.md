---
id: TASK-0583
title: >-
  TEST-19: subprocess env-override tests call set_var/remove_var without unsafe
  (edition-2024 hazard)
status: Triage
assignee: []
created_date: '2026-04-29 05:17'
labels:
  - code-review-rust
  - TEST
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/subprocess.rs:270`

**What**: Tests in mod env_override (lines 270, 279, 288, 296) call `std::env::set_var(...)` and `std::env::remove_var(...)` directly, relying on `#[serial]` for ordering. (a) When the workspace migrates to edition 2024 these become hard errors (UNSAFE-8); (b) other tests already use the unsafe form (expand.rs:220, results.rs:225) so the inconsistency itself is a maintenance smell.

**Why it matters**: TEST-19/UNSAFE-8 readiness. Adopt unsafe form now to avoid flag-day at edition migration.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each set_var/remove_var call wrapped in unsafe { ... } with SAFETY comment
- [ ] #2 No behaviour change; tests pass under cargo test and --test-threads=1
<!-- AC:END -->
