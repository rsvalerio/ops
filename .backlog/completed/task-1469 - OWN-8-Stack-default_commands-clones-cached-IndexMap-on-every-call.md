---
id: TASK-1469
title: 'OWN-8: Stack::default_commands clones cached IndexMap on every call'
status: Done
assignee: []
created_date: '2026-05-16 10:05'
updated_date: '2026-05-17 09:13'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/stack/mod.rs:116-122`

**What**: `default_commands` returns an owned `IndexMap<String, CommandSpec>` via `.cloned().unwrap_or_default()` — every call deep-clones every `CommandSpec` (with its nested `Vec<String>` args, env map, etc.) even though the cache is the authoritative copy.

**Why it matters**: The whole point of `default_commands_cache` (TASK-1409) is to avoid re-parsing TOML; the docstring explicitly notes the clone is O(n). Callers in `ops init` and config-merge paths that iterate or sample never needed ownership. The clone defeats the cache's value for read-only callers.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Add a borrowing accessor default_commands_ref(&self) -> &'static IndexMap<String, CommandSpec> (or Arc<IndexMap<...>>); keep the owning default_commands for callers that mutate
- [x] #2 Migrate read-only callers to the borrowed form
- [x] #3 Benchmark or assert via Arc::ptr_eq/identity that repeat calls do not allocate CommandSpec contents
<!-- AC:END -->
