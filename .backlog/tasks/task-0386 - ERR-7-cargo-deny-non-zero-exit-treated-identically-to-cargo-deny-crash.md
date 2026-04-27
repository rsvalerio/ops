---
id: TASK-0386
title: 'ERR-7: cargo deny non-zero exit treated identically to cargo-deny crash'
status: To Do
assignee:
  - TASK-0421
created_date: '2026-04-26 09:39'
updated_date: '2026-04-26 10:10'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/parse.rs:148`

**What**: run_cargo_deny notes "cargo deny exits non-zero when issues are found — that is expected" and unconditionally parses stderr. RunError::Io (cargo-deny missing) and RunError::Timeout are now indistinguishable from "ran successfully and reported issues". A non-zero exit due to internal cargo-deny error (e.g., bad config) is parsed as if it were an issue list and may yield zero entries.

**Why it matters**: A broken deny.toml looks like "no issues" instead of surfacing the configuration error.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Distinguish exit codes: cargo-deny uses 0=clean, 1=issues, 2=usage/config error — bail on 2
- [ ] #2 Test asserts that an invalid deny.toml surfaces an error rather than an empty DenyResult
<!-- AC:END -->
