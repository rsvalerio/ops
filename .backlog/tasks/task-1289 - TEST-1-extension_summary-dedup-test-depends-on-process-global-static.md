---
id: TASK-1289
title: 'TEST-1: extension_summary dedup test depends on process-global static'
status: To Do
assignee:
  - TASK-1304
created_date: '2026-05-11 16:10'
updated_date: '2026-05-11 16:48'
labels:
  - code-review-rust
  - tests
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/extension_cmd.rs:610-675`

**What**: `extension_summary_warn_is_dedup_per_cli_invocation` relies on the module-level static `warned_self_shadow_set` (a `OnceLock<Mutex<HashSet>>`) and never resets it. Another test in the same binary that inserts the same key `("dedupe_warn_ext","lint")`, or a re-run of this test in one process, makes the `warn_count == 1` assertion observe `0` instead.

**Why it matters**: Order-dependent assertion against module-level state. Flaky under `--test-threads=N` or future test reorderings; the test cannot be repeated within one process. Couples production logging behaviour to test isolation.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Factor warned_self_shadow_set behind an injectable parameter or per-call state so tests get a fresh set
- [ ] #2 Test does not depend on global state across the test binary
- [ ] #3 Running the test twice in a row within one process still passes
<!-- AC:END -->
