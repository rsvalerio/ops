---
id: TASK-0623
title: >-
  READ-4: Go lib.rs doc-claim about debug logging diverges from sister-crate
  behaviour
status: Triage
assignee: []
created_date: '2026-04-29 05:21'
labels:
  - code-review-rust
  - READ
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/lib.rs:7`

**What**: Crate-level docstring at extensions-go/about/src/lib.rs:7-8 says "non-NotFound read errors are reported via tracing::debug!". extensions-python/about/src/lib.rs:8-10 documents both debug! and warn!. extensions-node/about/src/lib.rs:7-9 matches Python. extensions-java/about/src/lib.rs:7-11 describes Maven as "DataProviderError::computation_failed" plus tracing::debug, which is now stale (TASK-0438 made Maven fall back via unwrap_or_default).

**Why it matters**: Diverging docstrings on parallel crates are documentation-form TASK-0467: readers infer different policies than the code implements.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 All four lib.rs files describe read-error/parse-error logging policy in identical wording (or centralised in ops_about)
- [ ] #2 Java lib.rs doc-claim about DataProviderError::computation_failed updated to match unwrap_or_default fallback
<!-- AC:END -->
