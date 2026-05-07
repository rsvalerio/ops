---
id: TASK-1119
title: >-
  OWN-8: project_identity card std_field_specs uses .clone().filter() instead of
  .as_ref().filter().cloned()
status: Done
assignee: []
created_date: '2026-05-07 22:08'
updated_date: '2026-05-07 23:19'
labels:
  - code-review-rust
  - ownership
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/project_identity/card.rs:30,56,61`

**What**: Three Option<String> fields (`license`, `repository`, `homepage`) are built with `id.<field>.clone().filter(|s| !s.is_empty())`. The clone is performed on the `Option<String>` (forcing a String allocation for the inner value) before the filter decides whether the value will even be kept. The idiomatic form is `id.<field>.as_ref().filter(|s| !s.is_empty()).cloned()`, which only clones when the value passes the filter.

**Why it matters**: OWN-8 — `.clone()` here exists to dodge a borrow rather than to express intent. Practical impact is small (the function runs once per `ops about` and the strings are short), but the pattern is repeated three times, makes the inner condition harder to read, and is exactly the kind of habit that becomes a real cost when copy-pasted into a hot path. It also masks the fact that the predicate is a borrow check (no allocation needed).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace each .clone().filter(...) with .as_ref().filter(...).cloned() at lines ~30, 56, 61
- [ ] #2 About card output unchanged under existing render tests
<!-- AC:END -->
