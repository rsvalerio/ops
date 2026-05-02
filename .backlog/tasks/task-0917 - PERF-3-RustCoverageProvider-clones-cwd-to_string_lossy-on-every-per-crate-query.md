---
id: TASK-0917
title: >-
  PERF-3: RustCoverageProvider clones cwd to_string_lossy on every per-crate
  query
status: Triage
assignee: []
created_date: '2026-05-02 10:12'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/coverage_provider.rs:63`

**What**: `let workspace_root = cwd.to_string_lossy();` allocates a Cow<str> for every provide() call even though cwd is already owned. The string is consumed by query_crate_coverage which takes &str.

**Why it matters**: Minor; flagged because the comment chain in this module emphasises avoiding redundant clones in the about hot path.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace the let-bind with an inline reference at the call site or take Cow<str>/Path through the helper
<!-- AC:END -->
