---
id: TASK-0046
title: 'TQ-2: query.rs has 4 public query functions with zero test coverage'
status: Triage
assignee: []
created_date: '2026-04-14 20:22'
labels:
  - rust-test-quality
  - TestGap
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
extensions-rust/about/src/query.rs — The functions query_loc_data, query_deps_data, query_deps_tree_data, and query_coverage_data have no direct tests. They are used by run_dashboard and RustIdentityProvider but never tested in isolation. Error paths (None returns, provider failures) are completely unexercised. resolve_member_globs has 3 tests and query_language_stats is indirectly tested via formatting tests. Rules: TEST-5, TEST-6.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each of the 4 query functions has at least one direct unit test
- [ ] #2 Error/fallback paths (None returns) are exercised
<!-- AC:END -->
