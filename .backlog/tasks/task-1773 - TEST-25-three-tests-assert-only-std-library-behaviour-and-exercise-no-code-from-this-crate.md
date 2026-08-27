---
id: TASK-1773
title: >-
  TEST-25: three tests assert only std library behaviour and exercise no code
  from this crate
status: Triage
assignee: []
created_date: '2026-08-27 11:22'
labels:
  - code-review-rust
  - test-quality
dependencies: []
modified_files:
  - extensions-rust/about/src/units.rs
  - extensions-rust/about/src/query.rs
  - extensions-rust/about/src/coverage_provider.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/units.rs:264-271`, `extensions-rust/about/src/query.rs:698-705`, `extensions-rust/about/src/coverage_provider.rs:414-432`

**What**: Three tests are framework-only under TEST-25 — they call no function defined in this crate and assert only documented `std` behaviour.

1. `units.rs:265 crate_metadata_breadcrumb_debug_escapes_control_characters` — builds a `Path`, formats `{:?}` of `p.display()`, asserts the result has no raw `\n` / ESC. Nothing from `units.rs` is invoked. This is a test of `std::fmt::Debug for std::path::Display`.
2. `query.rs:699 workspace_glob_path_debug_escapes_control_characters` — byte-for-byte the same test with a different literal (also TEST-12: the two are redundant with each other).
3. `coverage_provider.rs:415 non_utf8_cwd_path_to_str_returns_none` — asserts `Path::to_str()` returns `None` for invalid UTF-8 and that `to_string_lossy()` yields U+FFFD. Both are `std` guarantees; `coverage_provider`'s own short-circuit at `:133-142` is never reached.

Each carries a docstring claiming to protect a real invariant: *"tracing fields for crate-manifest paths flow through the `?` formatter"* and *"pin that invariant so a future refactor that swaps in `to_string_lossy` can't silently re-introduce the lossy-collapse."* Neither is what the test checks. Swapping `path = ?crate_toml_path.display()` for `path = %crate_toml_path.display()` at `units.rs:196`, or replacing the `to_str()` short-circuit at `coverage_provider.rs:133` with `to_string_lossy()`, leaves all three tests green — the exact regressions they were written to catch.

<!-- scan confidence: candidates to inspect -->
Candidates verified individually at the line numbers above; no other test in the crate matched.

**Why it matters**: These are the highest-risk form of dead test — green, documented, and named after a guarantee they do not provide. They inflate the crate's apparent coverage of the ERR-7/READ-5 log-forging and lossy-path defences while leaving those defences unpinned.

**Fix direction**: point each test at the crate's own code. For (1)/(2), install a `tracing_subscriber` over `ops_about::test_support::TracingBuf` (the pattern already used in `query.rs:955` and `coverage_provider.rs:224`), call `read_crate_metadata` / `expand_member_glob` with a control-character-bearing path, and assert on the *captured log line*. For (3), drive `RustCoverageProvider::provide` with a non-UTF-8 cwd and assert the provider returns default coverage plus the warn, rather than asserting on `Path` itself. Fold (1) and (2) into one helper if the assertion is genuinely identical (TEST-12).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each of the three tests invokes a function defined in this crate and asserts on its observable output (captured tracing log or provider return value), not on std Path/Debug behaviour
- [ ] #2 Changing the tracing field formatter from ? to % at units.rs:196 or query.rs:607 makes the corresponding test fail
- [ ] #3 Replacing the to_str() short-circuit in RustCoverageProvider::provide with to_string_lossy() makes the coverage_provider test fail
- [ ] #4 The two byte-identical control-character tests in units.rs and query.rs are deduplicated or given genuinely distinct subjects (TEST-12)
<!-- AC:END -->
