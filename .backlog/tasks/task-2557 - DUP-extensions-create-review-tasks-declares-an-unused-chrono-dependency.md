---
id: TASK-2557
title: 'DUP: extensions/create-review-tasks declares an unused chrono dependency'
status: To Do
assignee: []
created_date: '2026-10-10 15:40'
updated_date: '2026-10-10 21:12'
labels:
  - code-review
  - dependencies
dependencies: []
parent_task_id: 'TASK-2620'
modified_files:
  - extensions/create-review-tasks/Cargo.toml
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/create-review-tasks/Cargo.toml:13`

**What**: `chrono = { workspace = true }` is declared but no source or test file in the crate references chrono — all time handling goes through `ops_backlog::clock::UtcStamp`. Verified by grep over src/ and tests/ during the TASK-2351.24 review.

**Why it matters**: dead dependency inflates build graph and review surface for no benefit.

**Source**: TASK-2351.24 review of extensions/create-review-tasks (no code-review rule covers unused deps per-crate).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 chrono removed from extensions/create-review-tasks/Cargo.toml and cargo check -p ops-create-review-tasks still passes
<!-- AC:END -->
