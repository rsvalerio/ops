---
id: TASK-0242
title: 'DUP-3: three overlay structs duplicate the single-Option wrapper pattern'
status: Done
assignee: []
created_date: '2026-04-23 06:35'
updated_date: '2026-04-23 14:32'
labels:
  - rust-code-review
  - duplication
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/mod.rs:142`

**What**: ExtensionConfigOverlay/AboutConfigOverlay/DataConfigOverlay each is a struct with a single Option<T> field plus deny_unknown_fields; merge code repeats the same if-let-Some pattern.

**Why it matters**: Three places to update when adding new single-field overlay sections; invites drift.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract generic or macro single_field_overlay!(Name, field: Type)
- [ ] #2 Or collapse to a single overlay type with Option fields
<!-- AC:END -->
