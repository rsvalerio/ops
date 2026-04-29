---
id: TASK-0579
title: >-
  ERR-2: register_extension_commands collision detection misses self-collisions
  within one extension
status: Triage
assignee: []
created_date: '2026-04-29 05:17'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/registry.rs:140`

**What**: Per-extension `local: CommandRegistry` (IndexMap) iteration warns when a different extension already owns an id, but `IndexMap::insert` overwrites silently within local. If a single extension`s register_commands inserts the same id twice, the first registration is dropped before the loop sees both — warning never fires for self-collisions.

**Why it matters**: TASK-0402 fixed cross-extension shadowing visibility but left the within-extension self-shadow path silent.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Per-extension registration tracks duplicates and warns when register_commands re-uses the same id within itself
- [ ] #2 Test: fake extension that inserts X twice triggers a tracing warning
<!-- AC:END -->
