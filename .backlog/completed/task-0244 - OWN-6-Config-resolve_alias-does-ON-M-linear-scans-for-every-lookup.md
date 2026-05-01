---
id: TASK-0244
title: 'OWN-6: Config::resolve_alias does O(N*M) linear scans for every lookup'
status: Done
assignee: []
created_date: '2026-04-23 06:35'
updated_date: '2026-04-23 14:32'
labels:
  - rust-code-review
  - ownership
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/mod.rs:60`

**What**: Iterates all commands and all aliases per alias lookup; no cache.

**Why it matters**: Alias lookup is called per-CLI-invocation; grows with user configs.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Build an alias→canonical map once during load
- [ ] #2 Document complexity
<!-- AC:END -->
