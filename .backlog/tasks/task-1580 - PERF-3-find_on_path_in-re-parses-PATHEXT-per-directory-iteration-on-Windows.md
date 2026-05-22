---
id: TASK-1580
title: 'PERF-3: find_on_path_in re-parses PATHEXT per directory iteration on Windows'
status: Done
assignee:
  - TASK-1637
created_date: '2026-05-21 22:44'
updated_date: '2026-05-22 12:48'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/probe/path.rs:126-135` (call site) and `:140-148` (definition)

**What**: Inside `find_on_path_in`, the `for dir in std::env::split_paths(path_var)` loop calls `pathext_suffixes()` on every Windows iteration (`for ext in pathext_suffixes()`). `pathext_suffixes()` re-reads `PATHEXT` via `std::env::var_os`, splits it, and allocates a fresh `Vec<OsString>` each invocation. For a typical Windows `PATH` with dozens of directories, this re-runs the same `var_os` lookup and allocation N times per `find_on_path` call.

**Why it matters**: `check_binary_installed` is on the per-tool fallback path for `collect_tools`. The PATHEXT vector is invariant per process — hoist it once before the `split_paths` loop (or once per process via `OnceLock<Vec<OsString>>`), reducing the Windows probe cost from O(|PATH| × |PATHEXT|) env reads to O(1).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 pathext_suffixes() is invoked at most once per find_on_path_in call (or cached process-wide)
- [x] #2 Behavior preserved: missing PATHEXT still falls back to .COM;.EXE;.BAT;.CMD
- [x] #3 Existing path_index_case_tests still pass
<!-- AC:END -->
