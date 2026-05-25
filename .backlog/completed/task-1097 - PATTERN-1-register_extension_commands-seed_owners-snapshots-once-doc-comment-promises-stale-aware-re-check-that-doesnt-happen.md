---
id: TASK-1097
title: >-
  PATTERN-1: register_extension_commands seed_owners snapshots once; doc comment
  promises stale-aware re-check that doesn't happen
status: Done
assignee: []
created_date: '2026-05-07 21:32'
updated_date: '2026-05-07 23:30'
labels:
  - code-review-rust
  - pattern
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/registry/registration.rs:75-119`

**What**: `seed_owners` snapshots `registry.keys()` once before the loop. The seed for extension N+1 doesn't observe N's contributions — collisions classify via `owners.insert` at line 98, not via the seed. The comment at line 51 promises seed re-checking on re-entry, but the function only runs once.

**Why it matters**: Confusing doc comment plus a name (`seed_owners`) that suggests dynamic state — drift hazard for future maintainers; `register_extension_data_providers` (line 144) likely has the same shape.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either rename seed_owners to clarify it captures the *initial* state (e.g. seed_initial_owners) or compute it lazily
- [ ] #2 Audit register_extension_data_providers (line 144) for the same staleness shape
- [ ] #3 Pin behaviour with a unit test that registers three extensions all colliding on "build" and asserts the warn-source field at each step
<!-- AC:END -->
