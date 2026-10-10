---
id: TASK-2460
title: 'READ-13: workspace_root_cache module docs narrate how the memoization came to exist'
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
  - extensions-rust/about/src/workspace_root_cache.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions-rust/about/src/workspace_root_cache.rs:workspace_root_cache'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/workspace_root_cache.rs:3-14`

**What**: The module doc opens with the design history rather than the contract: "CL-3 / TASK-1762 keyed the typed-manifest cache by the *resolved* workspace root ... **That forced** the resolution to happen **before** the cache probe, so `find_workspace_root_strict` ... **ran on every** `load_workspace_manifest` call, cache hits included. Four providers hit the cache per `ops about` run ... so **the walk ran four times over** for one answer that cannot change between them." Past-tense narration of the pre-fix cost; the current contract (one walk per cwd, memoized) is buried under the story of why it was added.

**Why it matters**: READ-13: a reader using the module needs the cache contract (key, value, cap, invalidation — already documented below), not the migration essay; the history text would be deleted verbatim by someone who joined after the decision. This file was outside the TASK-2374 / TASK-2426 cleanup scopes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Module doc leads with the memoization contract in present tense; the 'that forced / ran on every / ran four times over' history is removed or recast as the enduring reason (the walk cannot change between providers within one run)
<!-- AC:END -->
