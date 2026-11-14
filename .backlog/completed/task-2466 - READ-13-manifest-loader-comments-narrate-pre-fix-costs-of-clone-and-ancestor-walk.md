---
id: TASK-2466
title: 'READ-13: manifest loader comments narrate pre-fix costs of clone and ancestor walk'
status: Done
assignee: []
created_date: '2026-10-10 15:27'
updated_date: '2026-10-10 21:35'
labels:
  - code-review
  - READ
dependencies: []
parent_task_id: 'TASK-2614'
modified_files:
  - extensions-rust/about/src/manifest.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions-rust/about/src/manifest.rs:resolve_workspace_root'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/manifest.rs:249-253`, `extensions-rust/about/src/manifest.rs:284-291`

**What**: Two comments surviving the TASK-2426 pass still narrate pre-fix costs:
- `resolve_workspace_root` doc (249-253): "because keying the typed-manifest cache by the resolved root ... put this walk *ahead* of the cache probe — so every provider's cache hit **paid for it again** against the same cwd." Past-tense account of the pre-memoization cost; the enduring reason is that the walk cannot change between providers within one run.
- `parse_manifest` comment (284-291): "instead of `(**cached).clone()` deep-cloning the entire tree ... **The clone allocated one Box per nested map/array node** — multi-MB workspaces clone 10k+ allocations **only to drop them**." The first sentence states the choice; the cost accounting of the rejected alternative is the fix's justification, not the contract.

**Why it matters**: READ-13: both passages make the reader reconstruct the old shape to extract the current invariant (root resolution is memoized per cwd; deserialization borrows the cached value). manifest.rs was in TASK-2426's scope, but these two items (`resolve_workspace_root`, `parse_manifest`) kept their journey text.

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 resolve_workspace_root doc states the memoization contract in present tense without 'paid for it again'
- [x] #2 parse_manifest comment states the borrow-based deserialization choice without the past-tense 'The clone allocated ... only to drop them' cost accounting

<!-- AC:END -->
