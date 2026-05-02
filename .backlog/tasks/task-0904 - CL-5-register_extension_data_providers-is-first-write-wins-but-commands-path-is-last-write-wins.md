---
id: TASK-0904
title: >-
  CL-5: register_extension_data_providers is first-write-wins but commands path
  is last-write-wins
status: Triage
assignee: []
created_date: '2026-05-02 10:09'
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
- [ ] #1 Document the explicit rationale for asymmetric wins (or unify both to last-write-wins)
- [ ] #2 If kept asymmetric, encode the policy in shared helper names (e.g. register_first_wins / register_last_wins) so call sites declare intent
- [ ] #3 Add a test pinning each registry's resolution policy under a colliding pair
<!-- AC:END -->
