---
id: TASK-0652
title: >-
  ARCH-9: CommandRegistry exposes DerefMut, bypassing duplicate_inserts
  bookkeeping
status: To Do
assignee:
  - TASK-0740
created_date: '2026-04-30 05:11'
updated_date: '2026-04-30 06:07'
labels:
  - code-review-rust
  - idioms
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/extension.rs:120-131`

**What**: `CommandRegistry` exposes its inner `IndexMap` via blanket `Deref`/`DerefMut` impls. Callers can route around `CommandRegistry::insert` (which carries the `duplicate_inserts` audit trail backing ERR-2 self-shadow warnings) via `*registry.insert(...)`, `entry().or_insert(...)`, etc., silently dropping duplicates without bookkeeping.

**Why it matters**: The whole point of the wrapper is undermined by `DerefMut`; the audit invariant only holds if every mutation goes through the wrapper.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Drop the DerefMut impl; expose only the read methods callers actually use (keys, iter, get, contains_key, len, is_empty)
- [ ] #2 Or move duplicate-tracking into a method that wraps IndexMap::insert and remove DerefMut so the only mutating path is CommandRegistry::insert
<!-- AC:END -->
