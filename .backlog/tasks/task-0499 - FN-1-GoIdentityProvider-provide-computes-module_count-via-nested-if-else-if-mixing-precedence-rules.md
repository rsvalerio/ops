---
id: TASK-0499
title: >-
  FN-1: GoIdentityProvider::provide computes module_count via nested if/else-if
  mixing precedence rules
status: To Do
assignee:
  - TASK-0532
created_date: '2026-04-28 06:10'
updated_date: '2026-04-28 07:14'
labels:
  - code-review-rust
  - fn
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/lib.rs:71`

**What**: The module_count = if let Some(ref work)... else if let Some(ref m)... else None block embeds two policies (workspace-takes-precedence; single-mod-with-replaces-only-counts-when-greater-than-1) inside provide(). It is the only non-trivial logic in provide and the rule is undocumented at the call site.

**Why it matters**: Reading provide() requires re-deriving why module_count differs between go.work, go.mod-with-replaces, and single-mod cases. Extracting compute_module_count(go_work, go_mod) -> Option<usize> with a doc comment captures the policy and matches the single-abstraction-level cleanup already done for sibling providers (TASK-0396).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract a private fn compute_module_count(go_work: Option<&GoWork>, go_mod: Option<&GoMod>) -> Option<usize>
- [ ] #2 Doc comment states the precedence rule explicitly
- [ ] #3 provide() body is a flat sequence of let-bindings and the build_identity_value call
<!-- AC:END -->
