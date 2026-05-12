---
id: TASK-1344
title: >-
  PATTERN-1: builtin_theme_names Box::leaks each derived String to satisfy a
  HashSet<&'static str> when a HashSet<String> would suffice
status: Done
assignee:
  - TASK-1384
created_date: '2026-05-12 16:41'
updated_date: '2026-05-12 23:23'
labels:
  - code-review-rust
  - patterns
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/theme_cmd.rs:42-50`

**What**: `builtin_theme_names` returns `&'static HashSet<&'static str>` and populates it by `Box::leak(name.into_boxed_str())` for every key parsed from the embedded default config. The only caller (`collect_theme_options`, line 57) does `builtins.contains(name.as_str())` — a `HashSet<String>` with the same `contains` call would satisfy the API without any leaking.

**Why it matters**: `Box::leak` on derived data is a code-smell future contributors may copy onto less-safe inputs (e.g. user-supplied theme names) where it becomes a slow leak. The set is OnceLock-initialised so it lives forever anyway — switching to `OnceLock<HashSet<String>>` is a one-line change with identical semantics and no leak.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 builtin_theme_names returns &'static HashSet<String>; no Box::leak
- [ ] #2 collect_theme_options compiles and tests pass under cargo test --workspace
<!-- AC:END -->
