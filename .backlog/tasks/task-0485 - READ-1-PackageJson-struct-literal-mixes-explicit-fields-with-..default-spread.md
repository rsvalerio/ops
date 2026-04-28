---
id: TASK-0485
title: >-
  READ-1: PackageJson struct literal mixes explicit fields with ..default()
  spread
status: To Do
assignee:
  - TASK-0532
created_date: '2026-04-28 06:08'
updated_date: '2026-04-28 07:14'
labels:
  - code-review-rust
  - read
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/package_json.rs:84-103`

**What**: The struct literal in parse_package_json explicitly assigns every field of PackageJson except authors, then ends with `..PackageJson::default()`. Authors is overwritten unconditionally on line 116, so the spread is dead syntax.

**Why it matters**: The `..default()` shorthand suggests fields rely on it, but none do. Future field additions silently default instead of forcing the call site to consider them. Listing every field exhaustively turns adding a field into a compile error here.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Remove ..PackageJson::default() from the literal in parse_package_json
- [ ] #2 Initialise authors: Vec::new() (or build authors before the literal) so the literal exhaustively lists every field
- [ ] #3 Adding a new field to PackageJson causes a compile error at this site
<!-- AC:END -->
