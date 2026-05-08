---
id: TASK-1179
title: >-
  API-9: DataRegistry::IntoIterator surfaces internal HashMap ordering as part
  of the public contract
status: To Do
assignee:
  - TASK-1269
created_date: '2026-05-08 08:09'
updated_date: '2026-05-08 13:19'
labels:
  - code-review-rust
  - api
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/data.rs:253`

**What**: `impl IntoIterator for DataRegistry` returns `std::collections::hash_map::IntoIter<...>`, exposing the underlying hashmap iteration order to callers. `provider_names()` sorts on output, but `IntoIterator` does not — downstream consumers iterating the registry will see non-deterministic ordering.

**Why it matters**: Hashbrown's randomized iteration means CLI wiring code that walks IntoIterator will produce non-reproducible warning ordering for take_duplicate_inserts flows. Pairs poorly with CommandRegistry, which is IndexMap-backed and does expose deterministic order via Deref.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 DataRegistry::IntoIterator yields entries in deterministic (insertion or sorted) order, matching the documented expectations of take_duplicate_inserts audit-trail consumers.
- [ ] #2 Test asserts iteration order is stable across two registries built from the same insertion sequence.
<!-- AC:END -->
