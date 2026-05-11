---
id: TASK-1294
title: 'PERF-1: gather_available_commands clones each command name twice'
status: Done
assignee:
  - TASK-1305
created_date: '2026-05-11 16:10'
updated_date: '2026-05-11 18:16'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/hook_shared.rs:117-166`

**What**: `gather_available_commands` clones every command `name: String` once into the `seen` `HashSet` (`seen.insert(name.clone())`) and again into the `SelectOption.name` field. With three near-identical loops this is two allocations per command across each loop.

**Why it matters**: Small but unnecessary double-alloc on every hook install. With sizeable command tables (50+ in larger workspaces) this doubles String allocations on a UX-facing path. Pairs with the existing DUP-1 finding on the same triple-loop pattern.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either build SelectOptions first and derive seen as &str view, or store Rc<str>/Arc<str> once and clone the handle
- [ ] #2 Allocation count on a 50-command config is roughly halved (verifiable via a counter or bench)
- [ ] #3 Existing DUP-1 dedupe of the triple loop is preserved or unblocked
<!-- AC:END -->
