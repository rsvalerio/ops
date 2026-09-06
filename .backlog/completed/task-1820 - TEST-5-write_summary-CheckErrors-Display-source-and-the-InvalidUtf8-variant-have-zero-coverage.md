---
id: TASK-1820
title: >-
  TEST-5: write_summary, CheckError's Display/source, and the InvalidUtf8
  variant have zero coverage
status: Done
assignee:
  - TASK-2004
created_date: '2026-08-27 11:33'
updated_date: '2026-08-28 22:25'
labels:
  - code-review-rust
  - test-quality
dependencies: []
modified_files:
  - extensions/config-checkers/src/lib.rs
  - extensions/config-checkers/src/json.rs
  - extensions/config-checkers/src/yaml.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/config-checkers/src/lib.rs:308-320` (`write_summary`), `lib.rs:74-102` (`CheckError`, its `Display` and `Error::source`), `extensions/config-checkers/src/yaml.rs:15` and `extensions/config-checkers/src/json.rs:17` (the `InvalidUtf8` construction sites)

**What**: the crate's tests (`lib.rs:322-485`, `yaml.rs:21-39`, `json.rs:28-54`) cover the happy paths and the two checkers well, but four public behaviours have no test at all:

1. **`write_summary` — zero tests.** It is public, it is the crate's only user-facing summary line, and it is what `crates/cli/src/subcommands.rs:356` prints on every run. Nothing asserts its text, its pluralisation, its field order, or that the three counters land in the right slots. A transposed `files_failed.len()` / `files_skipped` would pass the whole suite.

2. **`CheckError::InvalidUtf8` is never constructed in any test.** Both producers are untested: `check_yaml` on non-UTF-8 bytes (`yaml.rs:15`) and `check_json(bytes, allow_json5 = true)` on non-UTF-8 bytes (`json.rs:17`). This is a real asymmetry worth pinning down — strict JSON goes through `from_slice` and reports non-UTF-8 as a *parse* error, while the json5 branch reports it as `InvalidUtf8`, so the same input yields different variants depending on a flag. Nothing documents or tests that.

3. **`CheckError`'s `Display` and `Error::source`** (`lib.rs:86-102`) are hand-written — `source()` returns `Some` for `InvalidUtf8` and `None` for `Parse` — and neither arm is asserted. The existing error tests only check `!err.to_string().is_empty()`, which is TEST-11-weak and would pass on an empty-ish or wrong message.

4. **`tracked_only = true` is never exercised.** Every test constructs `CheckerOptions::new(root, false)`. The tracked path is the one with the sharpest behavioural differences (see the ERR-2 and SEC-25 findings on this file) and it has no coverage.

**Why it matters**: TEST-5 (every public API function needs at least one test) and TEST-6 (error paths, not just happy paths). `write_summary` in particular is a public function on the crate's output contract with no test whatsoever, and the summary line is the part a human actually reads.

**Fix shape**: assert `write_summary`'s exact rendered line for a report with a mix of scanned/failed/skipped counts (a snapshot via `insta` per TEST-30 suits the rendered-text shape). Add non-UTF-8 input tests for both checkers asserting the specific variant via `assert_matches!` (TEST-29), and assert `source()` is `Some` for `InvalidUtf8` / `None` for `Parse`. Add a `tracked_only = true` test over a scratch git repo.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 write_summary has a test asserting its exact rendered output for a report with non-zero scanned, failed, and skipped counts
- [x] #2 Both InvalidUtf8 producers (check_yaml, and check_json with allow_json5) have a non-UTF-8 input test asserting the CheckError variant, not just that an error occurred
- [x] #3 CheckError::source() is asserted for both variants (Some for InvalidUtf8, None for Parse)
- [x] #4 At least one test exercises CheckerOptions with tracked_only = true
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Fixed in wave TASK-2004.

AC#1: `tests::write_summary_reports_each_counter_in_its_own_slot` asserts the
exact rendered line for scanned 7 / failed 2 / skipped 3, which pins the
wording, the field order and that each counter lands in its own slot. Plain
string equality rather than `insta` — the crate has no snapshot dependency and
the line is one deterministic sentence, so a snapshot would add a dependency
and an accept step without adding a reviewable diff.

AC#2 — substitution recorded. The asymmetry the AC asked us to pin no longer
exists, and removing it was required by TASK-1830 in this same wave: dropping
`serde_json::Value` for `IgnoredAny` also drops serde_json's UTF-8 validation
of string bodies, so `check_json` now validates UTF-8 up front in *both* modes.
`json::tests::non_utf8_input_reports_invalid_utf8_in_both_modes` asserts
`CheckError::InvalidUtf8` for both flag values (and doubles as the regression
guard for that relaxation); `yaml::tests::non_utf8_input_reports_invalid_utf8`
covers the YAML producer.

AC#3: `json::tests::parse_errors_keep_the_underlying_parser_error_as_their_source`
asserts `source()` is `Some` for `Parse` and downcasts it to
`serde_json::Error`; `yaml::tests::invalid_utf8_exposes_the_utf8_error_as_its_source`
does the same for `InvalidUtf8`. Note `Parse` now returns `Some` rather than
`None` — that is TASK-1824's change, landed in this wave.

AC#4: three tracked-mode tests over scratch git repos
(`tracked_only_validates_the_files_git_lists`,
`tracked_but_deleted_file_is_skipped_rather_than_failing_the_hook`,
`tracked_symlink_to_a_character_device_is_skipped_not_read`). Each bails out if
git is unavailable, because `discovery::discover` would otherwise fall back to
the walk and the assertion would pass vacuously.
<!-- SECTION:NOTES:END -->
