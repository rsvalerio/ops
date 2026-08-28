---
id: TASK-1739
title: >-
  TEST-5: no test exercises any run_about_* entry point despite writer/is_tty
  seams built for that purpose
status: To Do
assignee:
  - TASK-2003
created_date: '2026-08-27 11:13'
updated_date: '2026-08-28 14:15'
labels:
  - code-review-rust
  - test-quality
dependencies: []
modified_files:
  - extensions/about/src/lib.rs
  - extensions/about/src/units.rs
  - extensions/about/src/coverage.rs
  - extensions/about/src/deps.rs
  - extensions/about/src/code.rs
  - extensions/about/src/loc.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/lib.rs:112` (`run_about`), `extensions/about/src/units.rs:50` (`run_about_units_with`), `extensions/about/src/coverage.rs:68` (`run_about_coverage_with`), `extensions/about/src/deps.rs:32` (`run_about_deps_with`), `extensions/about/src/code.rs:76` (`run_about_code_with`), `extensions/about/src/loc.rs:196` (`run_about_loc_with`)

**What**: Every one of the crate's six top-level entry points takes an injected `writer: &mut dyn Write` (and, for three of them, an explicit `is_tty` and/or `term_width`). Those parameters exist for exactly one reason — the doc comments say so: "Passing a `Vec<u8>` writer with `is_tty = false` guarantees no ANSI escapes regardless of stdout state" (`units.rs:35-39`), and "Buffer-writing call sites must hand in an explicit width ... so the 120-column fallback never sneaks into output destined for a `Vec<u8>` or pipe" (`units.rs:41-44`).

**No test in the repository calls any of them.** Verified: the crate has no `tests/` directory, and grepping the whole workspace for `run_about_units_with|run_about_coverage_with|run_about_deps_with|run_about_code_with|run_about_loc_with` finds only the definitions and the thin stdout wrappers in this crate; `crates/cli/src/subcommands.rs:34-50` calls the public wrappers in production, never from a test.

Every existing test in the crate targets a pure formatter or helper below these functions (`format_coverage_section`, `format_dependencies_section`, `format_rust_loc_section`, `render_card`, `resolve_member_globs`, ...). The composed behaviour is untested, so nothing pins:

- the "no output data" branches — `"No project units found."` (`units.rs:68`), `"No coverage data available."` (`coverage.rs:90`), `"No dependency data available."` (`deps.rs:48`), `"No Rust LOC data available."` (`loc.rs:209`) — each a user-facing string with no assertion behind it;
- the READ-5/TASK-0411 contract these signatures were introduced to guarantee: that a non-TTY writer receives zero ANSI escapes end to end (only `format_dependencies_section` has a section-level version of that assertion, not the runner);
- the ERR-1/TASK-0784 contract that `run_about_units_with` honours the caller-supplied `term_width` rather than probing;
- `run_about`'s fallback branch: `resolve_identity` returning `build_fallback_identity` on `NotFound`, and the four-condition `enrich_from_db` guard at `lib.rs:125-131`.

These are cheap to test — `warm_providers`/`load_or_default` already degrade to `Default` for unregistered providers, so an empty `DataRegistry` plus a `Vec<u8>` writer drives each runner through its empty branch in a handful of lines, and `providers.rs`'s existing `FailingProvider` test double covers the error branch.

**Why it matters**: TEST-5 (every public API function needs a test) with an aggravating factor — the seams that make these functions testable were added deliberately in three separate tasks and then never used. The contracts documented on those parameters are currently enforced by comment only, so a refactor that reads `stdout().is_terminal()` inside a runner, or drops the caller's `term_width`, passes the whole suite.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each run_about_*_with entry point has at least one test driving it with a Vec<u8> writer and an empty DataRegistry, asserting the exact no-data message it emits
- [ ] #2 At least one test asserts a non-TTY writer receives output containing no 0x1b byte, driven through a runner rather than a section formatter
- [ ] #3 run_about_units_with is tested with two different term_width values and the resulting cards-per-row differ, pinning that the caller-supplied width is used
- [ ] #4 run_about is tested for the NotFound fallback path, asserting the rendered card reflects build_fallback_identity
<!-- AC:END -->
