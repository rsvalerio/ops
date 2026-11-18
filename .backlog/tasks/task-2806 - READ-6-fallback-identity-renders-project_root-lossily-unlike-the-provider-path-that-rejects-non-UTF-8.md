---
id: TASK-2806
title: 'READ-6: fallback identity renders project_root lossily, unlike the provider path that rejects non-UTF-8'
status: Done
assignee: []
created_date: '2026-11-18 18:46'
updated_date: '2026-11-18 21:36'
labels:
  - code-review-rust
  - READ
dependencies: []
parent_task_id: 'TASK-2858'
modified_files:
  - extensions/about/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'READ-6:extensions/about/src/lib.rs:build_fallback_identity'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/lib.rs:284-293` (`build_fallback_identity`)

**What**: When no stack provider is registered, the about card builds its identity with `ProjectIdentity::new(name, stack_label, cwd.display().to_string(), "modules")`. `identity::build_identity_value` documents the opposite policy for the same field: a non-UTF-8 `cwd` is rejected with a typed error "rather than letting `Path::display` smuggle U+FFFD replacement bytes into the `project_root` field", so that every persisted/rendered root round-trips faithfully.

**Why it matters**: Two code paths producing the same `ProjectIdentity::project_root` follow different rules, so the guarantee the identity module documents holds only when a stack provider exists. READ-6 asks for one pattern for one problem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 build_fallback_identity applies the same non-UTF-8 policy as build_identity_value (shared helper or documented deliberate difference)
- [x] #2 A unix test covers a non-UTF-8 cwd on the fallback path

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.22 (code-review-rust run, 2026-11-18).

Shared identity::project_root_str; build_fallback_identity now returns Result.

<!-- SECTION:NOTES:END -->
