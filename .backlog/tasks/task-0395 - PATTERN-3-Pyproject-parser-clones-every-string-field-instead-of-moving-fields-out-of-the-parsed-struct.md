---
id: TASK-0395
title: >-
  PATTERN-3: Pyproject parser clones every string field instead of moving fields
  out of the parsed struct
status: To Do
assignee:
  - TASK-0417
created_date: '2026-04-26 09:40'
updated_date: '2026-04-26 10:10'
labels:
  - code-review-rust
  - patterns
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-python/about/src/lib.rs:60` (also extensions-node/about/src/lib.rs:59-69)

**What**: provide calls .clone() on name, version, description, license, requires_python, homepage, repository, and the authors Vec, even though parsed is a local owned Option<Pyproject> consumed nowhere else.

**Why it matters**: Each call clones owned Strings to satisfy the borrow checker against an Option<&Pyproject>. Per OWN-8/PERF-3, cloning to satisfy borrowing is a design smell — taking parsed by value eliminates ~8 allocations per call.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Refactor provide to consume parsed once via destructuring (let (name, version, ...) = match parsed { Some(p) => (p.name, p.version, ...), None => Default::default() }) — no .clone() needed
- [ ] #2 Apply the same destructuring to the Node provider (lib.rs:59-74)
<!-- AC:END -->
