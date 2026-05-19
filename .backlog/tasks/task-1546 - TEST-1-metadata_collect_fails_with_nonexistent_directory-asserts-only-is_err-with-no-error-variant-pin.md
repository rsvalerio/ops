---
id: TASK-1546
title: >-
  TEST-1: metadata_collect_fails_with_nonexistent_directory asserts only
  is_err() with no error-variant pin
status: Done
assignee:
  - TASK-1576
created_date: '2026-05-19 15:25'
updated_date: '2026-05-19 17:48'
labels:
  - code-review-rust
  - TEST
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/ingestor.rs:160-171`

**What**: The test body's sole assertion is `assert!(result.is_err());`. It does not distinguish between the intended failure mode (cargo exits non-zero because the working directory has no Cargo.toml) and unrelated failures (e.g. `RunError::Io` because `cargo` is not on `PATH`, `DbError::Io` from the unrelated `create_dir_all` step which actually runs first against the *data* directory rather than the missing working dir, or a `Timeout` if a slow CI subprocess hits 120s).

**Why it matters**: TEST-1 covers assertions that pass for the wrong reason. On a CI runner without `cargo` on `PATH` this test passes for the I/O failure, not for the "no manifest" failure it's named after. The fix is small: match on the `DbError` variant (e.g. assert it is `DbError::External` carrying a "cargo metadata" or "no such file" string, or assert the error's chain contains `cargo metadata`). Without that pin, the test is decorative — its failure tells the reviewer nothing about which boundary actually regressed.

<!-- scan confidence: candidates to inspect -->
Other Triage-worthy candidates in the same file (lower confidence, may be intentional):
- `metadata_collect_writes_atomically_no_tmp_leftover` (ingestor.rs:180) relies on a real `cargo metadata` succeeding in the test's manifest dir; the test passes if collect() panics-then-recovers, or if cargo silently produces zero output.
- `negative_record_count_surfaces_as_invalid_record_count_error` (ingestor.rs:586) is a tautology: it constructs a `try_from(-1)` failure and pattern-matches the error it constructed itself. It exercises no production code path.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 metadata_collect_fails_with_nonexistent_directory matches on the DbError variant or error chain so the failure cause is pinned to the missing manifest, not any I/O failure
- [ ] #2 negative_record_count_surfaces_as_invalid_record_count_error either exercises the production code path that surfaces the error, or is removed as a tautology
<!-- AC:END -->
