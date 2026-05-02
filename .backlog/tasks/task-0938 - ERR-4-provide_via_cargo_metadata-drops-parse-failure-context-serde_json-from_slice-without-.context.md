---
id: TASK-0938
title: >-
  ERR-4: provide_via_cargo_metadata drops parse-failure context
  (serde_json::from_slice without .context)
status: Done
assignee: []
created_date: '2026-05-02 15:51'
updated_date: '2026-05-02 17:26'
labels:
  - code-review-rust
  - errors
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/lib.rs:212`

**What**: `provide_via_cargo_metadata` calls `serde_json::from_slice(&output.stdout)?;` with the `?` operator and no `.context(...)`. When `cargo metadata` returns malformed JSON (rare, but observable when a custom registry shim or sccache wrapper corrupts stdout, or when stdout is truncated by an OOM), the error propagates as a bare `serde_json::Error` with no indication that it came from the cargo-metadata pipeline.

The sibling `test-coverage::collect_coverage` (test-coverage/src/lib.rs:251) attaches `.context("parsing llvm-cov JSON output")` for the same shape, and the project's ERR-4 rule requires `?` propagation across module boundaries to include `.with_context()` so the error chain remains attributable.

**Why it matters**: Operators investigating a failing `ops about` will see "expected value at line 1 column 1" with no breadcrumb that the offending payload was the cargo-metadata stdout. The same fix has been applied throughout the codebase (sweep TASK-0795/TASK-0913/TASK-0844). One-line fix; same shape as the test-coverage sibling.

<!-- scan confidence: candidates to inspect -->

- Candidate site: `extensions-rust/metadata/src/lib.rs:212` — `let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;`
- Sibling already-correct pattern: `extensions-rust/test-coverage/src/lib.rs:251`
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 serde_json::from_slice call wraps the result with .context() naming the cargo-metadata source
- [x] #2 Error chain on a malformed-JSON failure path mentions 'cargo metadata' (via test or manual repro)
<!-- AC:END -->
