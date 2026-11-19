---
id: TASK-2821
title: 'TEST-12: tokei_ingestor_load_without_collect_fails duplicates ingestor_load_errors_when_json_missing'
status: To Do
assignee: []
created_date: '2026-11-18 18:49'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - test
dependencies: []
parent_task_id: 'TASK-2855'
modified_files:
  - extensions/tokei/src/tests.rs
priority: low
ordinal: 1000
dedup_key: 'TEST-12:extensions/tokei/src/tests.rs:tokei_ingestor_load_without_collect_fails'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/tokei/src/tests.rs:674` (`tokei_ingestor_load_without_collect_fails`), duplicate of `extensions/tokei/src/tests.rs:569` (`ingestor_load_errors_when_json_missing`)

**What**: Both tests open an empty `IngestDir` and an in-memory DB, then call `TokeiIngestor::load` without a prior collect. The earlier test also asserts that the error is the missing-sidecar error. The later one asserts only `result.is_err()` (TEST-11), so it verifies a strict subset of the same path. The only difference is that it skips `init_schema`, and it does not assert anything that depends on that.

**Why it matters**: The redundant test adds maintenance and reading cost and no coverage. Its bare `is_err()` would also pass for an unrelated failure, such as a schema error.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 tokei_ingestor_load_without_collect_fails is removed, or changed to cover a distinct scenario (e.g. load on an uninitialised schema) with an assertion on the specific error
- [ ] #2 cargo test -p ops-tokei passes
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.32 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
