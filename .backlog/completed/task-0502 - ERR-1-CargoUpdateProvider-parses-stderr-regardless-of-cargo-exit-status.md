---
id: TASK-0502
title: 'ERR-1: CargoUpdateProvider parses stderr regardless of cargo exit status'
status: Done
assignee:
  - TASK-0533
created_date: '2026-04-28 06:50'
updated_date: '2026-04-28 18:00'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-update/src/lib.rs:259`

**What**: provide() treats any successful spawn as success: if `cargo update --dry-run` exits non-zero, run_cargo still returns Ok(Output) and the provider hands stderr to parse_update_output. A failed cargo invocation (lockfile lock, network error) yields a clean empty CargoUpdateResult.

**Why it matters**: Sister extensions (test-coverage::check_llvm_cov_output, metadata::check_metadata_output, deps::interpret_deny_result) all check output.status before parsing. The about page presents the empty result as "no updates available" even though the underlying command failed.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 provide() inspects output.status; non-zero exit returns DataProviderError with stderr tail
- [x] #2 Parse path remains unchanged on success; existing tests still pass
<!-- AC:END -->
