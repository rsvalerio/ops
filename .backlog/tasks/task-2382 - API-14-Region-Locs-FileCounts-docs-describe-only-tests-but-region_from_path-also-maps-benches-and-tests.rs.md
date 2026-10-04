---
id: TASK-2382
title: 'API-14: Region / Locs / FileCounts docs describe only tests/ but region_from_path also maps benches/ and tests.rs'
status: To Do
assignee: []
created_date: '2026-10-04 14:13'
updated_date: '2026-10-04 14:52'
labels:
  - code-review-rust
  - api
dependencies: []
parent_task_id: 'TASK-2420'
modified_files:
  - extensions-rust/loc/src/counter.rs
  - extensions-rust/loc/src/tests.rs
priority: low
ordinal: 1000
dedup_key: 'API-14:extensions-rust/loc/src/counter.rs:Region'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/loc/src/counter.rs:76-85`, `counter.rs:149-159`, `extensions-rust/loc/src/tests.rs:116-126`

**What**: `Region::Main` is documented as "everything outside `tests/` and `examples/`" and `Region::Test` as "Integration tests under `tests/`"; `FileCounts::test` says "(`tests/`)". `region_from_path` (counter.rs:206-219) also classifies any `benches/` component and any `tests.rs` file as `Test`, and (via `syn`) `#[cfg(test)]` items inside production files land in `FileCounts::test` too. The public docs therefore under-describe what a consumer receives. Related staleness: the `tests.rs` section header cites `lib.rs:120-126, lib.rs:135-144` line ranges that no longer match, and the unreadable-file test doc still says the failure comes from `read_to_string` although the code now uses `read_capped_source`.

**Why it matters**: Public docs on a pub module (`pub mod counter`) are the contract; a reader of the `Region` docs would wrongly expect benches to count as production.

<!-- scan confidence: doc-only mismatch, verified against region_from_path -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Region::Main/Test/Example and FileCounts field docs name benches/, tests.rs and cfg(test) items
- [ ] #2 Stale line-number references and the read_to_string mention in tests.rs are corrected or removed
<!-- AC:END -->
