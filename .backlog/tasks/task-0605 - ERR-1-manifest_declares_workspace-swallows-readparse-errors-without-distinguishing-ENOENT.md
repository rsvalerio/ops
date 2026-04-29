---
id: TASK-0605
title: >-
  ERR-1: manifest_declares_workspace swallows read+parse errors without
  distinguishing ENOENT
status: Triage
assignee: []
created_date: '2026-04-29 05:19'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/lib.rs:313`

**What**: Both read_to_string failure and TOML parse failure return false silently. A truncated/partially-written Cargo.toml (concurrent cargo new, network FS) makes the discovery walk skip a real workspace root and fall back to the next candidate, producing wrong root with zero log signal. Sister functions (read_crate_metadata per TASK-0377) distinguish NotFound from other errors.

**Why it matters**: Inconsistent with project error-classification convention. A flaky disk silently mis-roots the entire about/units/coverage stack.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Read errors other than NotFound emit tracing::debug or warn
- [ ] #2 Parse errors emit tracing::warn (matches read_crate_metadata)
<!-- AC:END -->
