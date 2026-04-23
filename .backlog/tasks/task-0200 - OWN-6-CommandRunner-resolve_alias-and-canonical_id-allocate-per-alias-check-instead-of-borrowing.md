---
id: TASK-0200
title: >-
  OWN-6: CommandRunner resolve_alias and canonical_id allocate per-alias-check
  instead of borrowing
status: Done
assignee: []
created_date: '2026-04-22 21:26'
updated_date: '2026-04-23 14:59'
labels:
  - rust-code-review
  - PERF
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/mod.rs:215-242` (canonical_id, resolve_alias).

**What**: `canonical_id` iterates `for (name, spec) in src` and on each iteration calls `spec.aliases().iter().any(|a| a == id)`. For the common-path "direct command name" this is fine (returns early on `exists_in_stores`). But for every lookup that falls through to alias search the function walks both non_config sources and every spec's aliases linearly. With N commands and A aliases, that is O(N·A) per resolve, and resolve is called from every `expand_to_leaves` call (which itself recurses into composites).

**Why it matters**: OWN-6 / PERF-12 nit. For small workspaces (10-20 commands) this is negligible; for larger configs with many extensions it adds up. Fix: build a lazy `HashMap<&str, &CommandId>` of (alias → canonical) at CommandRunner construction time and consult it in O(1). Existing `Config::resolve_alias` already uses a dedicated map for config aliases — apply the same treatment to stack + extension aliases.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Build an alias→canonical lookup map at CommandRunner::new and use it in resolve_alias/canonical_id
<!-- AC:END -->
