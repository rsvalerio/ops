---
id: TASK-1616
title: >-
  PERF-3: is_in_cargo_list / is_component_in_list do O(n) line scans per tool in
  collect_tools
status: Done
assignee:
  - TASK-1637
created_date: '2026-05-22 06:51'
updated_date: '2026-05-22 12:55'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/probe/cargo.rs:69-82`, `extensions-rust/tools/src/probe/rustup.rs:132-140`; consumed from `extensions-rust/tools/src/lib.rs:199-216`.

**What**: `collect_tools` captures `cargo --list` and `rustup component list --installed` once per sweep (good), but `is_in_cargo_list` / `is_component_in_list` then re-walk the captured stdout line-by-line for *every* tool entry. With `T` tools and `L` lines in each capture this is `O(T × L)` per sweep, even though the captures are immutable across the inner loop and a single pre-built `HashSet<&str>` would make each lookup `O(1)`.

**Why it matters**: `ops about` and `ops tools list` block on these probes; the very reason `collect_tools` exists in the form it does is the PERF-3 / TASK-1046 path-index amortisation. Leaving the cargo-list / rustup-components scans quadratic in `T` defeats the same optimisation for the two other capture sources. A `tools.toml` with 30 entries already pays 60 full stdout walks per sweep.

<!-- scan confidence: high; same loop body for both call sites -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Introduce a helper that turns the captured stdout into a HashSet<String> of normalised tokens (cargo: leading word per line minus builtin filter; rustup: stripped target triple) and have collect_tools build it once before the per-tool loop.
- [x] #2 Keep the existing &str-based is_in_cargo_list / is_component_in_list as the per-call probe fallback (when the precomputed capture is absent) so the public per-tool API is unchanged.
<!-- AC:END -->
