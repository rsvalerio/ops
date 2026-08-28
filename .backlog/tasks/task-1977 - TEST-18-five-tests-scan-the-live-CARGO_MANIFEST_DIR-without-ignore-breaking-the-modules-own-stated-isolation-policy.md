---
id: TASK-1977
title: >-
  TEST-18: five tests scan the live CARGO_MANIFEST_DIR without #[ignore],
  breaking the module's own stated isolation policy
status: To Do
assignee:
  - TASK-2012
created_date: '2026-08-27 15:55'
updated_date: '2026-08-28 14:18'
labels:
  - code-review-rust
  - test-quality
dependencies: []
modified_files:
  - extensions/tokei/src/tests.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/tokei/src/tests.rs:346`, `:383`, `:443`, `:470`, `:494`

**What**: The module doc at `tests.rs:1-12` sets a policy in capitals:

    Tests that scan the live workspace via env!("CARGO_MANIFEST_DIR") are
    non-deterministic (file counts depend on the working tree) and slow.
    Any such test MUST be gated behind
    #[ignore = "scans CARGO_MANIFEST_DIR; non-deterministic and slow (TEST-17)"]

Five tests break it. Each calls `env!("CARGO_MANIFEST_DIR")`, runs a full tokei walk of the live crate directory, and carries no `#[ignore]`:

- `load_tokei_succeeds_after_collect` (line 346) -- collect + load, asserts `record_count > 0` and `COUNT(*) > 0`
- `query_tokei_files_returns_json_array` (line 383) -- collect + load + query, asserts `!arr.is_empty()`
- `flatten_tokei_with_unrelated_prefix_keeps_full_path` (line 443) -- asserts `!arr.is_empty()`, then `file.starts_with('/')` on every record, which is a unix-only assumption in a test with no `#[cfg(unix)]` gate (contrast `relativize_path_replaces_invalid_utf8_with_replacement_char` at line 546, which does gate)
- `tokei_files_create_sql_with_real_json` (line 470) -- collect only, then asserts on the generated SQL, which does not depend on the scan at all
- `tokei_languages_view_aggregates_correctly` (line 494) -- collect + load, then `SELECT SUM(code) FROM tokei_files` into `i64`, which panics on an `Option<i64>` conversion if the table is ever empty

Five other tests in the same file (lines 80, 202, 236, 259, 278) are correctly gated with exactly the documented attribute, so the policy is real and selectively applied. The shared mutable fixture here is the working tree itself: every one of these assertions depends on the contents of `extensions/tokei/` at run time, so deleting or emptying a source file, or running against a checkout where the crate is pruned, turns a passing suite red for reasons unrelated to the change under test. The lack of a byte or count cap in `collect_tokei` (see the SEC-33 finding filed against `lib.rs`) is what makes the cost scale with the tree rather than being merely wasteful.

Note that none of these five tests actually needs the live tree. `tokei_provider_returns_valid_json_on_canned_dir` (line 50) already demonstrates the pattern the module doc prescribes -- a `tempfile::tempdir()` with one canned source file -- and yields deterministic counts the assertions could check exactly rather than settling for `> 0`.

**Why it matters**: TEST-18 -- tests must own their state. These share the repository working tree, so their outcome is a function of the checkout rather than of the code under test, and the assertions are consequently weakened to existence checks that would pass on almost any input. Because the policy is stated in the file and then violated five times, the next author has no way to tell which convention is current.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each of the five tests either builds its fixture in a tempdir with known contents or carries the documented ignore attribute
- [ ] #2 Tests converted to a tempdir fixture assert exact expected counts instead of greater-than-zero
- [ ] #3 tokei_files_create_sql_with_real_json no longer runs a workspace scan, since its assertions concern only the generated SQL
- [ ] #4 flatten_tokei_with_unrelated_prefix_keeps_full_path either drops its unix-only path assumption or is gated with cfg(unix)
- [ ] #5 cargo nextest run -p ops-tokei passes with no test reading CARGO_MANIFEST_DIR outside an ignored test
<!-- AC:END -->
