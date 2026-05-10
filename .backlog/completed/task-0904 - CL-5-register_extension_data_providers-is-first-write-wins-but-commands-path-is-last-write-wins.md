---
id: TASK-0904
title: >-
  CL-5: register_extension_data_providers is first-write-wins but commands path
  is last-write-wins
status: Done
assignee: []
created_date: '2026-05-02 10:09'
updated_date: '2026-05-02 14:50'
labels:
  - code-review-rust
  - structure
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/registry.rs:253`

**What**: register_extension_commands always calls registry.insert (later wins) while register_extension_data_providers only inserts on Entry::Vacant (earlier wins). Both warn loudly per TASK-0756, but the resolution policies are opposite — a colliding command swaps to extension B, a colliding data provider sticks with extension A.

**Why it matters**: Operators reasoning about precedence by analogy get the wrong answer for one of the two registries; ordering of compiled-in extensions silently determines which provider wins.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Document the explicit rationale for asymmetric wins (or unify both to last-write-wins)
- [ ] #2 If kept asymmetric, encode the policy in shared helper names (e.g. register_first_wins / register_last_wins) so call sites declare intent
- [x] #3 Add a test pinning each registry's resolution policy under a colliding pair
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Documented the asymmetric collision-resolution policy at the module level (commands → last-write-wins, data providers → first-write-wins) with explicit rationale: commands mirror IndexMap::insert and the long-standing extensions.enabled override behaviour; data providers are first-write-wins as the security-trusted default for a "content source". Added the explicit policy phrase to each pub fn docstring header. Strengthened the existing data-providers test to assert StubProvider("a") survives (not "b") and added register_extension_commands_pins_last_write_wins as the dual. AC#2 (rename to register_first_wins/register_last_wins) deferred — the function names already encode the registry kind, and the docstring header announces the policy in the first line.
<!-- SECTION:NOTES:END -->
