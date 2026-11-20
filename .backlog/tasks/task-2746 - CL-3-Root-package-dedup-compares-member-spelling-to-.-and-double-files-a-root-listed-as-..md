---
id: TASK-2746
title: 'CL-3: Root-package dedup compares member spelling to "." and double-files a root listed as "./"'
status: Done
assignee: []
created_date: '2026-11-18 18:38'
updated_date: '2026-11-18 22:27'
labels:
  - code-review-rust
  - CL
dependencies: []
parent_task_id: 'TASK-2849'
modified_files:
  - extensions-rust/create-review-tasks/src/provider.rs
priority: medium
ordinal: 1000
dedup_key: 'CL-3:extensions-rust/create-review-tasks/src/provider.rs:RustReviewTargetsProvider::provide'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/create-review-tasks/src/provider.rs:138` (`RustReviewTargetsProvider::provide`)

**What**: The hybrid-manifest guard that keeps the root package from being emitted twice is a raw string comparison, `members.iter().any(|m| m == ROOT_PACKAGE_PATH)` with `ROOT_PACKAGE_PATH = "."`. It silently assumes `resolved_workspace_members` spells the root member exactly `"."`. It does not always: `ops_about_rust::members::normalize_member` (extensions-rust/about/src/members.rs:250) strips `.` and empty segments, and when nothing is left it returns the *original* string. So a manifest with `[package]` plus `members = ["./", "xtask"]` (or `".//"`, which Cargo accepts as the root) resolves to the member `"./"`. The guard misses it, the root package is pushed at path `"."`, and the `"./"` member then resolves (via `root.join("./").join("Cargo.toml")`) to the same package name. `disambiguate_target_names` turns that collision into two distinct targets, `ops-lib (.)` and `ops-lib (./)`, so create-review-tasks files two review subtasks for one crate. The existing test `hybrid_manifest_does_not_duplicate_a_root_listed_as_member` covers only the literal `"."` spelling.

**Why it matters**: The correctness of the dedup rests on an implicit, undocumented invariant about another crate's output spelling. When that invariant breaks, the run files duplicate review subtasks for the root crate, and the disambiguation step hides the duplication instead of surfacing it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The root-member check compares by path meaning (segments with "." and empty segments removed; empty means root), not by the raw string "."; alternatively normalize_member canonicalises an all-dot member to "." and the provider relies on that documented contract
- [x] #2 A hybrid manifest listing the root as "./" (and ".//") yields the root package exactly once; covered by a test alongside hybrid_manifest_does_not_duplicate_a_root_listed_as_member

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.14 (code-review-rust run, 2026-11-18).

Took the AC #1 alternative: normalize_member now emits the exported ROOT_MEMBER ('.') for every segment-less literal ('./', './/', ''), and the provider's ROOT_PACKAGE_PATH is defined as that constant.

<!-- SECTION:NOTES:END -->
