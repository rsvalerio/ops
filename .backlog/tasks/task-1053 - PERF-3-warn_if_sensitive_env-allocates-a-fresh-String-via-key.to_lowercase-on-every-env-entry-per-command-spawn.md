---
id: TASK-1053
title: >-
  PERF-3: warn_if_sensitive_env allocates a fresh String via key.to_lowercase()
  on every env entry per command spawn
status: Done
assignee: []
created_date: '2026-05-07 21:03'
updated_date: '2026-05-07 23:36'
labels:
  - code-review-rust
  - PERF
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/secret_patterns.rs:41` (and `:66` for `is_sensitive_env_key`)

**What**: `warn_if_sensitive_env` is called from `build_command_with` for every env-var pair on every spawn (`crates/runner/src/command/build.rs:364`). The first thing it does is `let lower = key.to_lowercase();`, which allocates a brand-new `String` for every key, then `lower.contains(pattern)` is checked against ~10 ASCII patterns.

**Why it matters**: env keys are almost always ASCII (`PATH`, `HOME`, `CARGO_HOME`, `AWS_*`, ...). The allocation is purely so `.contains` is case-insensitive. The same byte-level case-fold can be done with `eq_ignore_ascii_case` against a per-pattern `key.len() == pattern.len()` filter, or — for the substring case — by walking once over `key.as_bytes()` and matching ASCII-lower against each pattern. Either avoids the per-key allocation. Under a parallel plan with `OPS_MAX_PARALLEL=32` and a typical 30-entry env block, that's ~960 short-string allocations per second of steady-state command spawn just to feed the substring check.

**Comparison**: TASK-0301 (SEC-16) closed the unbounded *value* scan; the *key* scan still pays per-spawn allocation. Mirrors the per-spawn allocation pattern fixed in TASK-0764 / TASK-0838 for capture buffers.

**Severity rationale**: Low — the allocations are short and bounded; this surfaces a hot-path String allocation that the existing performance discipline elsewhere in the runner has already removed. Strict OWN-8 / PERF-3 nit.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace key.to_lowercase() with an ASCII-aware substring matcher that does not allocate (e.g. iterate key.as_bytes() and pattern.as_bytes() with eq_ignore_ascii_case slicing),Apply the same fix to is_sensitive_env_key,Pin a regression test that constructs an Env with >100 entries and asserts no per-spawn String allocation (or at least that warn_if_sensitive_env returns the same decision for upper/lower/mixed case input)
<!-- AC:END -->
