---
id: TASK-2464
title: 'READ-13: deps_provider provide comment narrates pre-fix silent-empty-deps behaviour'
status: To Do
assignee: []
created_date: '2026-10-10 15:27'
updated_date: '2026-10-10 21:12'
labels:
  - code-review
  - READ
dependencies: []
parent_task_id: 'TASK-2614'
modified_files:
  - extensions-rust/about/src/deps_provider.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions-rust/about/src/deps_provider.rs:RustDepsProvider::provide'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/deps_provider.rs:36-38`

**What**: The comment in `RustDepsProvider::provide` narrates the bug history: "ERR-2 / TASK-0376: a SQLite schema/migration error here **used to surface as an empty deps list with no signal**." The current contract is the positive half of the same sentence: query failures route through `query_or_warn` so they warn before falling back.

**Why it matters**: READ-13: the "used to surface" clause is a process artifact of the ERR-2 fix; the reader of the end state only needs the invariant. `deps_provider.rs` was outside the TASK-2374 / TASK-2426 cleanup scopes, which covered the sibling providers (units, coverage) for the same ERR-2 / TASK-0376 comment shape.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Comment states the query_or_warn contract in present tense; the 'used to surface as an empty deps list' clause is removed
<!-- AC:END -->
