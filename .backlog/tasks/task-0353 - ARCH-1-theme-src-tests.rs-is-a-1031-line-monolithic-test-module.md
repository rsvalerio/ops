---
id: TASK-0353
title: 'ARCH-1: theme/src/tests.rs is a 1031-line monolithic test module'
status: Done
assignee:
  - TASK-0418
created_date: '2026-04-26 09:35'
updated_date: '2026-04-27 10:32'
labels:
  - code-review-rust
  - architecture
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/tests.rs:1`

**What**: A single test file collects rendering, resolution, deserialization, error-block colour, summary, format_duration, left-pad, boxed-layout, and edge-case-width tests in 1031 lines with several inline submodules.

**Why it matters**: ARCH-1 flags >500-line modules mixing unrelated concerns. Splitting along existing inline mod boundaries lowers cognitive load.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Split the file along the existing inline mod boundaries into separate test files under crates/theme/src/tests/
- [ ] #2 Each new file stays under ~300 lines and groups one concern
<!-- AC:END -->
