---
id: TASK-1180
title: >-
  PERF-3: list_command_ids allocates Vec<&str>, sorts, dedups, then re-allocates
  Vec<CommandId>
status: To Do
assignee:
  - TASK-1263
created_date: '2026-05-08 08:09'
updated_date: '2026-05-08 13:18'
labels:
  - code-review-rust
  - perf
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/resolve.rs:152`

**What**: `list_command_ids` builds `ids: Vec<&str>` (line 153), sorts and dedups, then walks the deduped result to clone each `&str` into a fresh `CommandId` (line 156). A `BTreeSet<&str>` followed by a single mapping pass would dedup-and-sort in one shot.

**Why it matters**: Called by --list and the help/discovery paths. Not on the spawn hot path, but the allocation pattern is wasteful and the function is exercised in tab-completion latency.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 One pass collects sorted, deduped CommandId values without an intermediate Vec<&str>.
- [ ] #2 Output ordering / dedup behaviour unchanged.
<!-- AC:END -->
