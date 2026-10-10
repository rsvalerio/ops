---
id: TASK-2577
title: 'TEST-3: ops-run-before-push public-API tests live inline in src/ instead of tests/'
status: To Do
assignee: []
created_date: '2026-10-10 15:43'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - tests
dependencies: []
parent_task_id: 'TASK-2624'
modified_files:
  - extensions/run-before-push/src/lib.rs
priority: medium
ordinal: 1000
dedup_key: 'TEST-3:extensions/run-before-push/src/lib.rs:tests'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-push/src/lib.rs:295`

**What**: The `#[cfg(test)] mod tests` (627 lines incl. docs, vs 294 lines of production code) holds roughly ten tests that exercise only the public API and per TEST-3 belong in `tests/`:

- `classify_runs_when_stream_is_absent`
- `classify_reports_nothing_to_push_for_empty_stream`
- `classify_runs_for_a_normal_ref_update`
- `classify_reports_delete_only_when_every_local_oid_is_zero`
- `classify_runs_when_a_delete_is_mixed_with_an_update`
- `classify_runs_for_malformed_lines`
- `skip_reason_is_none_only_for_run`
- `skip_reason_reads_the_forwarded_env_var`
- `push_refs_runs_when_the_truncation_marker_is_set`
- `should_skip_returns_false_by_default`

Only the tests that genuinely need private access (`HOOK_SCRIPT`, `HOOK_CONFIG`, `MAX_REF_UPDATE_LINES` — the hook-script suite, `extension_constants`, `hook_config_*`, `classify_runs_when_the_stream_exceeds_the_line_bound`) must stay in `#[cfg(test)]`. The shared `SHA1_A`/`SHA1_B`/`ZERO` consts move with the classify tests; the dev-dependencies (serial_test, tempfile, ops-hook-common test-helpers) already work from `tests/`.

**Why it matters**: Integration tests prove the API is usable from outside the crate, and keeping src/ readable matters because the inline module currently outweighs the logic it covers 2:1 — both points are TEST-3's stated rationale. Moving the env-mutating tests to their own integration-test binary also shrinks the set of tests that must share the `#[serial_test::serial]` key with the child-spawning hook-script tests.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Public-API-only tests moved to a tests/ integration test; private-access tests remain in #[cfg(test)]
- [ ] #2 cargo test -p ops-run-before-push passes; serial_test key coverage preserved for tests that spawn children or mutate the environment
<!-- AC:END -->
