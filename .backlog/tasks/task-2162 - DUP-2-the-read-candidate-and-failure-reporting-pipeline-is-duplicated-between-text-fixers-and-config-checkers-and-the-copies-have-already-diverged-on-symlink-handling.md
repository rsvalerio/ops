---
id: TASK-2162
title: >-
  DUP-2: the read-candidate and failure-reporting pipeline is duplicated between
  text-fixers and config-checkers, and the copies have already diverged on
  symlink handling
status: Done
assignee:
  - TASK-2237
created_date: '2026-09-08 07:05'
updated_date: '2026-09-08 16:06'
labels:
  - code-review-rust
  - duplication
  - architecture
dependencies: []
modified_files:
  - extensions/text-fixers/src/runner.rs
  - extensions/text-fixers/src/report.rs
  - extensions/text-fixers/src/options.rs
  - extensions/config-checkers/src/runner.rs
  - extensions/config-checkers/src/report.rs
  - extensions/config-checkers/src/lib.rs
priority: medium
ordinal: 75000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/text-fixers/src/runner.rs:214-310`, `extensions/text-fixers/src/report.rs:8-64`
(twins: `extensions/config-checkers/src/runner.rs:88-279`, `extensions/config-checkers/src/report.rs:1-63`)

**What**: `ops-config-checkers` already depends on `ops-text-fixers` and calls
`ops_text_fixers::discovery::discover` — but only discovery was shared. The
whole layer above it exists twice, near-verbatim:

| Item | text-fixers | config-checkers |
|---|---|---|
| `read_candidate` | `runner.rs:224` | `runner.rs:192` |
| `open_regular_file` | `runner.rs:229` | `runner.rs:203` |
| `read_bounded` | `runner.rs:274` | `runner.rs:245` |
| `metadata_failure` / `read_failure` | `runner.rs:297,301` | `runner.rs:266,270` |
| `relative_to` | `runner.rs:305` | `runner.rs:276` |
| `record_failure` | `runner.rs:186` | `runner.rs:142` |
| walk-error print + `report.walk_errors = …` loop | `runner.rs:96-107` | `runner.rs:88-97` |
| `SkipReason` enum + `Display` | `report.rs:15-38` | `runner.rs:159-179` |
| `FailureKind` / `FailedFile` | `report.rs:42-58` | `report.rs:16-30` |
| `failed()` (+ identical doc comment) | `report.rs:100` | `report.rs:60` |
| `DEFAULT_MAX_BYTES = 16 * 1024 * 1024` | `options.rs:18` | `lib.rs:47` |

The comment blocks are copied too, down to the `max_bytes + 1` rationale and
the "The stat above was a snapshot" paragraph.

**They have already diverged, and in the security-relevant direction.**
text-fixers' `open_regular_file` uses `std::fs::symlink_metadata` so a symlink
is judged as itself; config-checkers' uses `std::fs::metadata`, which follows
the link and judges the target. text-fixers' `SkipReason` gained a fourth
variant (`NotText`); config-checkers' still has three. text-fixers' summary
line prints the counters in a different order from config-checkers'.

`DEFAULT_MAX_BYTES` is the same shape as TASK-2132: `options.rs:18` documents
"16 MiB matches `ops-config-checkers`' `DEFAULT_MAX_BYTES` so the two
file-walking extensions agree on what 'too big to hold' means" — an invariant
asserted only in prose, with the two constants free to drift.

**Why it matters**: both crates rewrite or gate the user's repository from a
pre-commit path, so a hardening fix landed in one copy silently does not reach
the other — which is exactly what happened with `symlink_metadata`. Every
future SEC-25 / TOCTOU / cap fix has to be applied twice and reviewed twice,
and nothing tells the reviewer the twin exists. The dependency edge needed to
share it is already in `Cargo.toml`.

Note that the shared code currently living inside `ops-text-fixers` is
consumed by a sibling extension, which is itself an ARCH-4 smell: a generic
file-walking / bounded-read concern is being re-exported from whichever
extension happened to write it first. Consider extracting it to its own crate
(or a module in `ops-core`) rather than deepening the extension-to-extension
dependency.

<!-- scan confidence: verified — both files read in full and compared -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The bounded read pipeline (open_regular_file / read_bounded / read_candidate) exists once and is used by both extensions
- [x] #2 SkipReason, FailureKind, FailedFile, relative_to, record_failure and the walk-error accounting loop exist once
- [x] #3 DEFAULT_MAX_BYTES is defined once and referenced by both crates, so the two cannot drift
- [x] #4 The shared code lives somewhere both extensions can depend on without one extension depending on the other (new crate or ops-core module), or the extension-to-extension edge is explicitly justified
- [x] #5 config-checkers picks up the symlink_metadata behaviour as part of the unification, with a test pinning that a symlinked config file is not judged by its target
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Shared layer lives in the new ops_core::bounded_read module: read_candidate/open_regular_file/read_bounded (text-fixers symlink_metadata semantics as canonical), SkipReason/FailureKind/FailedFile/Rejected, relative_to, record_failure + report_walk_errors over a FileRunReport trait implemented by FixerReport and CheckerReport, and DEFAULT_MAX_BYTES. Both extensions re-export the vocabulary so their public API names (ops_text_fixers::SkipReason, ops_config_checkers::FailureKind, DEFAULT_MAX_BYTES) are unchanged for the CLI. AC #4 taken on both branches: generic pipeline moved to ops-core (no extension-to-extension edge), while the remaining ops-text-fixers edge is only discovery, explicitly justified in config-checkers/lib.rs header (policy-coupled to the rewriting tools; moving it would pull the ignore crate into core for one consumer; extraction to its own crate noted as the move if a third consumer appears). AC #5: symlink-not-judged-by-target pinned by ops-core test a_symlink_is_never_judged_by_its_target (an end-to-end config-checkers test is unreachable: discovery drops symlinks in both modes before the runner, so the pipeline-level test is the closest meaningful pin). Skip wording pinned by skip_reason_wording_is_pinned. Clippy -D warnings clean on all three crates; 33+71 tests green.
<!-- SECTION:NOTES:END -->
