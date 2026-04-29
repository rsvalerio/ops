---
id: TASK-0580
title: >-
  PATTERN-3: register_extension_commands clones every CommandId twice per
  extension
status: Done
assignee:
  - TASK-0645
created_date: '2026-04-29 05:17'
updated_date: '2026-04-29 17:44'
labels:
  - code-review-rust
  - PATTERN
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/registry.rs:142`

**What**: The merge loop does `for (id, spec) in local { ... registry.insert(id.clone(), spec); owners.insert(id, ext.name()); }`. Each iteration moves id into owners, so registry.insert(id.clone(), spec) is needed — but reordering or using Entry could avoid the clone.

**Why it matters**: PATTERN-3/OWN-8: minor allocation per command per extension on every CLI startup. Similar to TASK-0200/0244.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Merge loop inserts each CommandId into both maps with at most one clone (or zero, via Entry)
- [ ] #2 Behaviour unchanged: collision warnings still fire
<!-- AC:END -->
