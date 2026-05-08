---
id: TASK-1046
title: >-
  PERF-3: tools collect_tools re-walks PATH per Cargo tool when cargo --list
  misses standalone binaries
status: Done
assignee: []
created_date: '2026-05-07 20:54'
updated_date: '2026-05-07 23:36'
labels:
  - code-review-rust
  - perf
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/lib.rs:107-139` (`collect_tools`), `extensions-rust/tools/src/probe.rs:160-200` (`check_binary_installed` / `find_on_path_in`)

**What**: `collect_tools` already amortises the heavy probes by capturing `cargo --list` and `rustup component list --installed` once (TASK-0830-class fix encoded in `check_tool_status_with`). However the secondary fallback path — `is_in_cargo_list(s, name) || check_binary_installed(name)` in `probe.rs:371` — re-walks `$PATH` for every Cargo-source tool that does *not* appear in `cargo --list` (any tool installed standalone via `cargo install`, e.g. `tokei`, `bacon`, `cargo-watch` shipped as a bare binary).

For a `[tools]` table with N entries that fall to the binary-on-PATH fallback, the loop is **O(N × |PATH| × |PATHEXT|)**:

* `find_on_path` reads `$PATH` once per call.
* `find_on_path_in` walks every directory in `$PATH` and, on Windows, every entry in `$PATHEXT`.
* `check_executable` calls `std::fs::metadata` (and possibly `symlink_metadata`) for each candidate.

A workspace with ~20 such tools and a typical 30-entry PATH performs ~600 stat syscalls per `ops about` invocation. The cost is invisible because each individual probe is fast, but it scales with workspace + PATH growth and undoes the amortisation `capture_cargo_list` was meant to deliver.

A single PATH-index pass (read `$PATH` once, glob each directory once with `read_dir`, build a `HashSet<OsString>` of executable names) would convert the inner loop to O(1) lookups, matching the `cargo --list` / `rustup component list --installed` amortisation pattern already in place.

**Why it matters**: PERF-3 — a hot-path probe that visibly contradicts its own amortisation rationale. Operator-facing latency on `ops about` / `ops tools list` scales with `[tools].len() × $PATH.len()` rather than `[tools].len()`, and the bigger the workspace gets the worse it scales. The fix slots in alongside the existing capture functions.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 collect_tools captures a one-shot PATH index (HashSet of executable basenames) and threads it into check_tool_status_with so each binary lookup is O(1) instead of an O(|PATH|) walk per tool
- [ ] #2 check_binary_installed accepts an optional precomputed PATH index and falls back to the per-call walk when None (preserving the public API for one-off callers)
<!-- AC:END -->
