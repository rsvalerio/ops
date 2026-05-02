---
id: TASK-0877
title: >-
  API-3: DataRegistry::provider_names allocates Vec<&str> when callers typically
  only iterate
status: Done
assignee: []
created_date: '2026-05-02 09:24'
updated_date: '2026-05-02 11:00'
labels:
  - code-review-rust
  - api-design
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/data.rs:193-197`

**What**: provider_names allocates a Vec<&str>, sorts in place, and returns it. Callers that just want to iterate (the common case for cargo ops data list) pay an allocation they do not need.

**Why it matters**: API-3 prefers impl Iterator when callers do not index/own. PERF-1 / PERF-2 align: sort-then-iterate via a BTreeMap-keyed registry would also remove the per-call sort.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Add pub fn provider_names_iter(&self) -> impl Iterator<Item = &str> returning sorted names
- [ ] #2 Or store providers in BTreeMap<String, ...> so iteration is sorted by construction
- [ ] #3 Keep provider_names for callers that need a Vec for indexing
<!-- AC:END -->
