---
id: TASK-0512
title: 'PERF-1: register_extension_commands snapshots every CommandId per extension'
status: To Do
assignee:
  - TASK-0536
created_date: '2026-04-28 06:51'
updated_date: '2026-04-28 07:14'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/registry.rs:132`

**What**: `let before: HashSet<CommandId> = registry.keys().cloned().collect();` clones every existing CommandId on every iteration; with N extensions registering M commands each, this is O(N^2 * M) clones just to detect duplicates.

**Why it matters**: Same diagnostic could be achieved by recording owners as commands are inserted (already done partially via `owners`) without snapshotting the registry every loop.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Drop the HashSet snapshot; rely on owners map for collision detection
- [ ] #2 Bench scales linearly with extension count
<!-- AC:END -->
