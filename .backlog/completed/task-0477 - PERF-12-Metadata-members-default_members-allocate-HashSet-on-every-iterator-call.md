---
id: TASK-0477
title: >-
  PERF-12: Metadata members/default_members allocate HashSet on every iterator
  call
status: Done
assignee:
  - TASK-0533
created_date: '2026-04-28 05:47'
updated_date: '2026-04-28 17:58'
labels:
  - code-review-rust
  - PERF
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/types.rs:135-298`

**What**: members() and default_members() invoke collect_member_ids (allocates HashSet<&str>) every time the iterator is constructed; same for is_member() / is_default_member() (full scan via id_in_field). Repeat callers (e.g. package_by_name followed by is_member) pay this each time.

**Why it matters**: A Metadata instance is held as Arc<Value> and reused; computing the workspace member id-set lazily-once (OnceCell or precomputed field) is a single-line change with measurable savings on workspaces with hundreds of packages.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Cache the workspace_members id set on Metadata (e.g. OnceLock<HashSet<String>>) and have members() / is_member() consult it
- [x] #2 Same caching for workspace_default_members
<!-- AC:END -->
