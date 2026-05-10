---
id: TASK-0874
title: >-
  ARCH-9: CommandRegistry exposes Deref<Target=IndexMap>, undermining
  minimal-public-surface invariant
status: Done
assignee: []
created_date: '2026-05-02 09:23'
updated_date: '2026-05-02 10:56'
labels:
  - code-review-rust
  - api-design
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/extension.rs:158-163`

**What**: The doc comment claims "only Deref is exposed so the audit trail in duplicate_inserts cannot be bypassed", but Deref to IndexMap exposes every read method including iterators, key counts, lookups by hash. While reads cannot bypass the audit trail, the type API is now an open IndexMap for read purposes.

**Why it matters**: Anyone reading the type will treat it as an IndexMap; refactoring later to swap the inner storage becomes a breaking change because all IndexMap methods are part of the de-facto public API.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace Deref with explicit accessors (get, iter, len, contains_key, keys)
- [ ] #2 Or document the Deref choice as intentional API surface and add a stability note
- [ ] #3 Keep IntoIterator impls
<!-- AC:END -->
