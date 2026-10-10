---
id: TASK-2605
title: 'API-13: test_support re-exports ops_core::test_utils items under a second public path'
status: To Do
assignee: []
created_date: '2026-10-10 20:50'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - api
dependencies: []
parent_task_id: 'TASK-2623'
modified_files:
  - extensions/about/src/test_support.rs
priority: low
ordinal: 1000
dedup_key: 'API-13:extensions/about/src/test_support.rs:test_support'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/test_support.rs:31`

**What**: `pub use ops_core::test_utils::{capture_tracing, capture_warn, count_warnings, pin_global_dispatcher, TracingBuf, WarnCounter};` gives six foreign items a second public alias, `ops_about::test_support::*`, while `ops_core::test_utils` remains the canonical public path. The re-export is `#[cfg(feature = "test-support")]`-gated, but every consumer of the feature (the about-family crates) already depends on `ops-core` and could import from `ops_core::test_utils` directly.

**Why it matters**: API-13: a public item should be reachable by exactly one path; foreign types come from their own crate. The alias doubles every mention of these helpers in docs and search results, and the (well-written) module doc must spend 20 lines justifying why the second path exists — a sign the re-export fights the rule.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 About-family test modules import the tracing-capture harness from ops_core::test_utils directly
- [ ] #2 The re-export (and its justification docs) is removed, or the re-export is kept with a triaged decision recorded on this task
<!-- AC:END -->
