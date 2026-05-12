---
id: TASK-1366
title: >-
  TEST-25: register_extension_data_providers_warns_on_in_extension_duplicate
  asserts the warning but never the registry shape
status: Done
assignee:
  - TASK-1383
created_date: '2026-05-12 21:29'
updated_date: '2026-05-12 23:16'
labels:
  - code-review-rust
  - tests
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/registry/tests.rs:413`

**What**: The test verifies the warn-log line for an in-extension duplicate data-provider registration, but never asserts the registry's post-state. The sibling command-path test (line 158) does assert `registry.len() == 1`; the symmetry is missing here.

**Why it matters**: TEST-25: test name says "duplicate is surfaced" but a regression that emitted the warning while also corrupting the registry (kept both, kept neither, last-write-wins) would still pass. First-write-wins (CL-5 / TASK-0756) needs to be pinned on the data-provider path too.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 After the call, assert registry contains exactly ["provider_x"] and registry.get("provider_x") is Some(_) — pinning first-write-wins
- [ ] #2 Match the shape of the parallel command-path duplicate test for symmetric coverage
<!-- AC:END -->
