---
id: TASK-0474
title: >-
  ARCH-1: extensions-rust/cargo-toml/src/tests.rs is a 1363-line monolithic test
  module
status: To Do
assignee:
  - TASK-0533
created_date: '2026-04-28 05:47'
updated_date: '2026-04-28 07:14'
labels:
  - code-review-rust
  - ARCH
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/tests.rs:1`

**What**: Single 1363-line test file covers parse, inheritance resolution, glob handling, and provider behaviour — distinct concerns. Echoes TASK-0423 (runner command tests) which the workspace already flagged as a smell.

**Why it matters**: ARCH-1 (god-module) — though test code, the organization concern still applies.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Decompose cargo-toml/src/tests.rs into per-concern test modules mirroring the production split (inheritance, types, lib)
- [ ] #2 No test logic change; only re-organization, with each new module under 500 lines
<!-- AC:END -->
