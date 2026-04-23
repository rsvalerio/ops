---
id: TASK-0161
title: >-
  TEST-18: merge_conf_d_applies_overlays mutates process cwd without serial
  guard
status: Done
assignee: []
created_date: '2026-04-22 21:23'
updated_date: '2026-04-23 14:29'
labels:
  - rust-code-review
  - TEST
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: crates/core/src/config/loader.rs:192-196

**What**: merge_conf_d_applies_overlays calls std::env::set_current_dir(dir.path()) and restores via std::env::set_current_dir(original), mutating process-global working directory without #[serial] or an EnvGuard-style scope. Other tests in this workspace run in parallel (cargo test) and any test that reads relative paths, reads env vars, or opens relative files during the window when cwd is swapped can observe the wrong directory.

**Why it matters**: Flaky tests and hard-to-reproduce CI failures. The top-of-file doc block notes env-mutating tests use #[serial]; cwd is equally process-global and should use the same discipline (or a dedicated CwdGuard helper).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Serialize the test with #[serial] (or isolate it via subprocess / scoped cwd guard)
- [ ] #2 Document the cwd-mutation discipline alongside the existing env-var note
<!-- AC:END -->
