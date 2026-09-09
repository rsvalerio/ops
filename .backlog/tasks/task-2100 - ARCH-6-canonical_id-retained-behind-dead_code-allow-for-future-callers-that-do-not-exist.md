---
id: TASK-2100
title: 'ARCH-6: canonical_id retained behind dead_code allow for future callers that do not exist'
status: To Do
assignee: []
created_date: '2026-09-08 06:40'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - architecture
dependencies: []
parent_task_id: 'TASK-2246'
modified_files:
  - crates/runner/src/command/resolve.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/resolve.rs:155-178`

**What**: `CommandRunner::canonical_id` is annotated `#[allow(dead_code)]` with the stated reason that it is "preserved as part of the public-ish helper surface that tests and future callers may depend on". No caller exists: production code routes through `canonical_with_spec` (TASK-0766), and the only remaining reference to the name is a prose mention in a test comment (`command/tests/expand.rs:754`). It is `pub(super)`, so it is not even reachable outside the crate.

**Why it matters**: ARCH-6 / YAGNI — code kept for a hypothetical second consumer is the speculative-flexibility shape the rule names. The function duplicates the store-walk chain that `canonical_with_spec` and `resolve_alias` each already own, so it is also a third copy of the config → stack → extension → alias precedence that can drift from the other two (its inline doc admits it survives only for "future callers"). AGENTS.md: "Minimum code that solves the problem. Nothing speculative." The traversal-counting test seam (`record_store_walk`) it carries is still used by the live paths, so removing the function removes no test coverage.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 canonical_id is deleted, or the allow is removed by giving the function a real caller (test or production) in the same change
- [ ] #2 If deleted, the config/stack/extension/alias precedence chain exists in exactly two places (canonical_with_spec, resolve_alias)
- [ ] #3 ops verify / ops qa gates pass
<!-- AC:END -->
