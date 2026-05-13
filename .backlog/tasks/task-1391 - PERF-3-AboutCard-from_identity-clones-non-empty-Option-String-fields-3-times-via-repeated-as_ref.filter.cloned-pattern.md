---
id: TASK-1391
title: >-
  PERF-3: AboutCard::from_identity clones non-empty Option<String> fields 3+
  times via repeated as_ref().filter().cloned() pattern
status: Done
assignee:
  - TASK-1452
created_date: '2026-05-13 18:03'
updated_date: '2026-05-13 20:35'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/project_identity/card.rs` (around the `from_identity_filtered` builder, fields like `license`, `repository`, `homepage`, `description`)

**What**: The card-construction code repeats `id.<field>.as_ref().filter(|s| !s.is_empty()).cloned()` for several `Option<String>` fields, allocating a fresh `String` for each non-empty optional even though the values land in an owned `Vec<(String, String)>` and could be moved if the identity were taken by value.

**Why it matters**: Minor per-call cost, but on the `ops about` rendering path (which builds the card for every invocation) the pattern repeats per field, and the duplicated idiom is a DUP-3 drift hazard — a future tightening (e.g. trim()-ing empties, normalising whitespace) has to be replicated at every callsite or it silently skips a field.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Introduce a small helper non_empty_clone(opt: &Option<String>) -> Option<String> (or non_empty(opt: Option<&str>) -> Option<String>) and replace every as_ref().filter(|s| !s.is_empty()).cloned() call in card.rs with it
- [ ] #2 Alternative if API-1 allows: have from_identity_filtered take ProjectIdentity by value so the helper can return owned strings without cloning; document the choice in the rustdoc
- [ ] #3 Add a unit test that asserts an Option<String> containing only whitespace or empty is treated as None by the helper, so future tightening can land in one place
<!-- AC:END -->
