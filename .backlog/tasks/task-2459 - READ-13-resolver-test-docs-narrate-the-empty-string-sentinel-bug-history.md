---
id: TASK-2459
title: 'READ-13: resolver test docs narrate the empty-string sentinel bug history'
status: To Do
assignee: []
created_date: '2026-10-10 15:26'
updated_date: '2026-10-10 21:12'
labels:
  - code-review
  - READ
dependencies: []
parent_task_id: 'TASK-2614'
modified_files:
  - extensions-rust/about/src/identity/resolver.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions-rust/about/src/identity/resolver.rs:resolve_field_falls_back_to_workspace_when_package_omits_the_key'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/identity/resolver.rs:156-165`

**What**: The doc comment on `resolve_field_falls_back_to_workspace_when_package_omits_the_key` narrates the journey, not the end state: "`InheritableField::default()` used to be `Value(\"\")`, so `p.version.as_str()` returned `Some(\"\")` ... and the `.or_else(...)` fallback never fired — `about` reported an empty version and description instead of the workspace's values" and "the `authors` arm ... had to hand-roll a `!wp.authors.is_empty()` guard for exactly this reason". A reader must reconstruct the pre-fix shape to understand what the test pins.

**Why it matters**: READ-13: process narration in docs goes stale on the next change while looking authoritative; the test doc should state the invariant the test pins (a `[package]` that exists but omits the field falls through to `[workspace.package]`; a declared empty string is kept). The TASK-2374 / TASK-2426 cleanup passes covered manifest/manifest_cache/members/units/coverage_provider but not `identity/resolver.rs`, so this file kept its history narration.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Test doc states the fallback invariant and the empty-string-beats-workspace case in present tense, with no 'used to be'/'never fired'/'had to hand-roll' narration
<!-- AC:END -->
