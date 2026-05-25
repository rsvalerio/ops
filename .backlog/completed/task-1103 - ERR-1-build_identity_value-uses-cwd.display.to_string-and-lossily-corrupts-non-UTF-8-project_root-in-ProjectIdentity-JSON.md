---
id: TASK-1103
title: >-
  ERR-1: build_identity_value uses cwd.display().to_string() and lossily
  corrupts non-UTF-8 project_root in ProjectIdentity JSON
status: Done
assignee: []
created_date: '2026-05-07 21:34'
updated_date: '2026-05-08 12:00'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/identity.rs:110`

**What**: `build_identity_value` writes `cwd.display().to_string()` into the `project_root` field of every stack's `ProjectIdentity`. `Path::display` replaces non-UTF-8 bytes with `U+FFFD`, so a project located under a non-UTF-8 path silently produces an identity payload whose `project_root` cannot be round-tripped to disk.

**Why it matters**: Contradicts the strict `DbError::NonUtf8Path` posture `upsert_data_source` adopted in TASK-0928 — corrupted root flows into downstream consumers (audit logs, JSON dumps) without operator signal.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 build_identity_value either rejects non-UTF-8 cwd with a typed error or stores the path via as_encoded_bytes so it round-trips faithfully
- [x] #2 A test using OsStr::from_bytes(b"/ws/\xff/proj") (Unix) on build_identity_value either errors or yields a project_root whose bytes match the input
- [x] #3 Document the policy alongside upsert_data_source's NonUtf8Path so the two paths share the same contract
<!-- AC:END -->

## Implementation Notes

- `extensions/about/src/identity.rs`: `build_identity_value` now calls `cwd.to_str().ok_or_else(|| DataProviderError::computation_failed(...))?` before constructing the `ProjectIdentity`, replacing the lossy `cwd.display().to_string()`. Doc comment cross-references the `upsert_data_source` `NonUtf8Path` contract.
- Added `#[cfg(unix)]` test `build_identity_value_rejects_non_utf8_cwd` using `OsStr::from_bytes(b"/ws/\xff/proj")` that asserts `DataProviderError::ComputationFailed`.
- `extensions/duckdb/src/schema.rs`: `upsert_data_source` doc now cross-references `ops_about::identity::build_identity_value` to record the shared fail-fast policy.
- Verified: `cargo fmt`, `cargo clippy --all-targets -p ops-about -- -D warnings`, `cargo test -p ops-about --lib` (100 passed including new test), and `ops verify` all green.
