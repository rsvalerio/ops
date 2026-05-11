---
id: TASK-1273
title: >-
  DUP-1: gather_available_commands repeats insert/push pattern across three
  loops
status: To Do
assignee:
  - TASK-1305
created_date: '2026-05-11 06:37'
updated_date: '2026-05-11 16:48'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/hook_shared.rs:117-166`

**What**: `gather_available_commands` walks three command sources (config.commands, stack.default_commands(), cmd_registry) with three near-identical loops that each (a) skip when name matches `exclude_name`, (b) check `seen.contains`, (c) `seen.insert(name.clone())`, and (d) push a `SelectOption { name, description: command_description(spec) }`. The dedup bookkeeping and the push body are repeated three times with only the source iterator changing.

**Why it matters**: Future changes (e.g. adding a 4th source, changing dedup keys, or normalizing names) have to be applied in three places, and the first loop already differs subtly from the next two — it does not check `seen.contains` because it runs first, so the invariant that drives correctness is implicit. Consolidating the loop body into a single helper closes that gap and makes the priority order (config > stack > extensions) explicit at the call site rather than implicit in the loop ordering.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 gather_available_commands collapses the three loops into a single loop over a priority-ordered iterator of (name, spec) sources, or into one helper invoked three times with no duplicated dedup/push body
- [ ] #2 Behavior is unchanged: config.commands wins over stack defaults, which win over extension-registered commands; exclude_name still skipped; existing tests pass without modification
<!-- AC:END -->
