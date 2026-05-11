---
id: TASK-1279
title: >-
  DUP-1: BufWriter + MakeWriter test scaffold duplicated 5 times in registry
  tests
status: To Do
assignee:
  - TASK-1304
created_date: '2026-05-11 15:25'
updated_date: '2026-05-11 16:48'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/registry/tests.rs:220-236, 285-301, 391-407, 525-541, 658-674`

**What**: The same ~17-line BufWriter helper (struct + Write impl + MakeWriter impl) is duplicated verbatim five times across tests that need to capture tracing output. Each copy is identical aside from being scoped locally to its test function.

**Why it matters**: Five copies of the same nontrivial test fixture inflate the file (~85 redundant lines), invite drift if one copy is tweaked, and obscure the actual test logic. A single shared `tracing_capture()` helper would shrink each test to its assertion meat.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract a single BufWriter/MakeWriter helper (or capture_warnings(|| ...) -> String) into a test-only helper module accessible from registry/tests.rs
- [ ] #2 Each of the five tests uses the shared helper
- [ ] #3 No behavioural change: all five tests continue to assert on the same captured substrings
<!-- AC:END -->
