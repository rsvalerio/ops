---
id: TASK-2591
title: 'TEST-32: tokei_default_excluded_contains_expected_dirs asserts the constant back against its own literals'
status: Done
assignee: []
created_date: '2026-10-10 15:46'
updated_date: '2026-10-10 21:58'
labels:
  - code-review-rust
  - test
dependencies: []
parent_task_id: 'TASK-2626'
modified_files:
  - extensions/tokei/src/tests.rs
priority: low
ordinal: 1000
dedup_key: 'TEST-32:extensions/tokei/src/tests.rs:tokei_default_excluded_contains_expected_dirs'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/tokei/src/tests.rs:431-437`

**What**: The test builds a `HashSet` from `TOKEI_DEFAULT_EXCLUDED` and asserts it contains `target`, `.git`, `node_modules`, `.venv` — a subset pin of a config constant against literals. It can only fail if someone edits the list, and the failure carries no information the behavioral test does not: `collect_tokei_excludes_target_and_git` (`:149-177`) already creates `noise.rs` inside exactly those four directories and asserts none of them are counted. The constant is asserted back to itself (TEST-32); the four remaining defaults (`venv`, `dist`, `build`) have no behavioral coverage at all.

**Why it matters**: TEST-32 — the assertion exists and is still worthless: it verifies that the code equals itself while looking like coverage. The meaningful contract — directories in the exclusion list are not scanned — is testable behaviorally.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The constant-subset assertion is removed
- [x] #2 Every entry of TOKEI_DEFAULT_EXCLUDED (including venv, dist, build) is pinned behaviorally: a fixture source file inside each excluded root-children dir is asserted absent from collect_tokei output

<!-- AC:END -->
