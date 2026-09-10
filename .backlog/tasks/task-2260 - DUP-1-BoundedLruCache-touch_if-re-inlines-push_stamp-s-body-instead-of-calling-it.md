---
id: TASK-2260
title: 'DUP-1: BoundedLruCache::touch_if re-inlines push_stamp''s body instead of calling it'
status: Triage
assignee: []
created_date: '2026-09-10 18:41'
labels:
  - code-review-rust
  - duplication
dependencies: []
modified_files:
  - extensions/about/src/lru.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/lru.rs:296` (`touch_if`), duplicating `:270` (`push_stamp`)

**What**: `BoundedLruCache::push_stamp` owns the victim-queue stamp-and-compact step. `touch_if` re-inlines that body rather than calling it, so the compaction threshold and the stamp push are written twice inside the same type, ~25 lines apart. The inline is borrow-checker driven — `touch_if` holds a borrow of the map entry across the stamp — not a deliberate policy split, and nothing in the code says so.

**Why it matters**: the whole point of TASK-2150 was to make the victim-queue policy exist once. Two copies inside the shared type itself is the same drift risk at smaller scale: a change to the compaction threshold or the stamp representation has to be made in both, with no test that would catch updating only one.

Bounded fix: restructure `touch_if` so the map borrow ends before the stamp (e.g. compute the decision, drop the borrow, then call `push_stamp`), or extract the shared tail into a small private helper that takes the already-resolved key. If the borrow really cannot be released, a comment on `touch_if` stating why the inline is required would at least keep the duplication intentional.

**Origin**: discovered during TASK-2242 (wave 8) while auditing TASK-2150; introduced by that wave's own fix.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The victim-queue stamp-and-compact step is written once in BoundedLruCache, or the inline is documented with the borrow constraint that forces it
<!-- AC:END -->
