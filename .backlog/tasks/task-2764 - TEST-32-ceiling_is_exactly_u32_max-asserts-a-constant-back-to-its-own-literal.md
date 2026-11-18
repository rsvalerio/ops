---
id: TASK-2764
title: 'TEST-32: ceiling_is_exactly_u32_max asserts a constant back to its own literal'
status: Done
assignee: []
created_date: '2026-11-18 18:42'
updated_date: '2026-11-18 21:47'
labels:
  - code-review-rust
  - TEST
dependencies: []
parent_task_id: 'TASK-2853'
modified_files:
  - extensions-rust/metadata/src/tests/payload_cap.rs
  - extensions-rust/metadata/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'TEST-32:extensions-rust/metadata/src/tests/payload_cap.rs:ceiling_is_exactly_u32_max'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/tests/payload_cap.rs:205`, `extensions-rust/metadata/src/lib.rs:137`

**What**: `METADATA_MAX_BYTES_CEILING` is declared as the literal `4_294_967_295` (lib.rs:137). The test `max_bytes_env::ceiling_is_exactly_u32_max` asserts `METADATA_MAX_BYTES_CEILING == u64::from(u32::MAX)`, so it re-states the constant's value. It passes by construction and tests no behaviour; that behaviour is already covered by `above_ceiling_warns_and_clamps`.

**Why it matters**: TEST-32 says a test must not assert ground truth back to itself. The test only adds diff churn. The intent ("the ceiling is u32::MAX bytes") is better expressed in the declaration itself, e.g. `u32::MAX as u64` with a scoped lint allow, or a `const _: () = assert!(...)`. That would make the test redundant.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 METADATA_MAX_BYTES_CEILING is expressed in terms of u32::MAX at its declaration (or a compile-time assert)
- [x] #2 The tautological ceiling_is_exactly_u32_max test is removed; clamping behaviour remains covered by above_ceiling_warns_and_clamps

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.18 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
