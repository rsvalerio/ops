---
id: TASK-0653
title: >-
  TRAIT-4: CommandRegistry derive(Clone) duplicates audit trail in
  duplicate_inserts
status: Done
assignee:
  - TASK-0740
created_date: '2026-04-30 05:12'
updated_date: '2026-04-30 19:12'
labels:
  - code-review-rust
  - idioms
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/extension.rs:88-92`

**What**: `#[derive(Clone)]` on `CommandRegistry` clones `duplicate_inserts` along with the map. That field is a per-registration audit trail consumed once by `take_duplicate_inserts`; cloning ships the audit history with the data so a downstream `clone()` user reading duplicates twice gets phantom warnings.

**Why it matters**: Either the `Clone` is wrong or the duplicate tracking does not belong in `Clone`-able state.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Remove Clone derive (no production caller clones the registry today), or implement Clone manually to reset duplicate_inserts to empty
- [ ] #2 Add a doc comment pinning whichever invariant you choose
<!-- AC:END -->
