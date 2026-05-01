---
id: TASK-0700
title: 'OWN-8: Variables::from_env allocates unnecessarily on every CLI invocation'
status: Done
assignee:
  - TASK-0741
created_date: '2026-04-30 05:27'
updated_date: '2026-04-30 19:34'
labels:
  - code-review-rust
  - OWN
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/expand.rs:61-66`

**What**: `from_env` builds a fresh `HashMap<String, String>` with two `String::from`/`display().to_string()` allocations on every invocation, called once per command spec expansion path (build_command, expanded_args_display, etc.). The two builtins (`OPS_ROOT`, `TMPDIR`) are invariant per process; `TMPDIR` resolution (`std::env::temp_dir`) is itself a syscall. Storing them as `Cow<'static, str>` or computing a `OnceLock<Variables>` keyed by ops_root would avoid the per-call allocation.

**Why it matters**: A typical `ops <cmd>` invocation expands ~5-20 args and calls `from_env` repeatedly. Not a hot path on the order of milliseconds, but the pattern (`String::from(...)` for known-static keys) is exactly the OWN-8 / PERF-3 anti-pattern flagged elsewhere in the workspace (TASK-0658 composite_tree_has_parallel, TASK-0655 merge_config double-clone).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Use &\'static str keys (HashMap<&\'static str, String> or two named fields)
- [x] #2 Avoid recomputing temp_dir per call (OnceLock or lazy field)
- [x] #3 Benchmark before/after to confirm no regression
<!-- AC:END -->
