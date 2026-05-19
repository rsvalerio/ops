---
id: TASK-1539
title: >-
  DUP-1: Metadata::package_index_by_name and package_index_by_id share ~25 lines
  of HashMap-build scaffolding
status: Done
assignee:
  - TASK-1576
created_date: '2026-05-19 15:23'
updated_date: '2026-05-19 17:48'
labels:
  - code-review-rust
  - DUP
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/types.rs:158-218`

**What**: `package_index_by_name` (lines 158-183) and `package_index_by_id` (lines 193-218) are structurally identical: both iterate `inner["packages"].as_array().into_iter().flatten().enumerate()`, both extract a string field (`"name"` vs `"id"`), both emit a `tracing::warn!` on collision with the same first-write-wins semantics, and both insert into a `HashMap<String, usize>`. The only deltas are the field name (`"name"` vs `"id"`) and the warn message wording.

**Why it matters**: This is the textbook DUP-1 pattern — two ~25-line functions that differ only in a string field and a log message. A future change (e.g. switching to `IndexMap` for deterministic iteration order, normalising names case-insensitively, or upgrading the warn to a structured-event style) requires editing both. A single private helper `fn build_package_index_by(&self, field: &str, dup_msg_field: &str) -> HashMap<String, usize>` would collapse the duplication; the two existing `OnceLock`-fronted callers become one-liners that pass `"name"` or `"id"`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 package_index_by_name and package_index_by_id share a single helper that takes the field name as an argument
- [ ] #2 The first-write-wins semantics and per-duplicate single-warn behaviour pinned by TASK-1019 and TASK-1100 remain unchanged
- [ ] #3 Existing duplicate-warning tests (tests.rs:1090, 1188) still pass without modification
<!-- AC:END -->
