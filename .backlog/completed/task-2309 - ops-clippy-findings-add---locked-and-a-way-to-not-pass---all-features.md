---
id: TASK-2309
title: 'ops clippy-findings: add --locked and a way to not pass --all-features'
status: Done
assignee: []
created_date: '2026-09-27 13:08'
updated_date: '2026-09-27 13:14'
labels:
  - feature
  - cli
  - clippy
  - skills-integration
dependencies: []
modified_files:
  - crates/cli/src/clippy_findings_cmd.rs
  - crates/cli/src/args.rs
priority: high
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**What**: `ops clippy-findings` hard-codes its cargo arguments (`crates/cli/src/clippy_findings_cmd.rs:42`: `clippy --workspace --all-features --all-targets --message-format=json`). Everything after `--` goes to clippy-driver as lint flags, so a caller cannot add cargo flags. Two are missing:

1. **`--locked`**: without it, a missing or stale `Cargo.lock` is silently written during resolution. The `rust-make-clippy-pedantic` skill promises the repository stays byte-identical, and passes `--locked` on every run for exactly that reason. If Cargo refuses (`the lock file needs to be updated`), the survey stops and reports it instead of rewriting the lock.
2. **Feature selection**: `--all-features` is always on. The pedantic skill adds it only when the workspace has no mutually exclusive features, and retries without it on a feature conflict. Also, a default-features survey is the other half of what the clippy gate misses (see ops' own `clippy-default`).

**Why it blocks adoption**: `rust-make-clippy-pedantic` (rsvalerio/ai) is the one consumer this command was built for (TASK-2283). Without these two flags it cannot switch from its own `cargo clippy | jq` extraction to `ops clippy-findings`.

**Sketch**: `--locked` (probably worth making the default for a survey, with `--no-locked` to opt out), and `--features <list>` / `--no-default-features` / `--no-all-features`, mirroring cargo. The scratch target dir already works through the inherited `CARGO_TARGET_DIR`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 ops clippy-findings can run cargo with --locked, and a stale or missing Cargo.lock then fails the survey without writing the lock
- [x] #2 A caller can run the survey without --all-features (default features, or an explicit feature list)
- [x] #3 Tests pin the cargo argument list for each flag combination

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
ops clippy-findings now passes --locked by default (--no-locked opts out); --no-all-features surveys default features; --features <list> (comma-separated or repeated) and --no-default-features select features explicitly and drop --all-features. Flags live in args::ClippyFindingsArgs, mapped via SurveyOptions::from_flags; clippy_args tests pin the cargo argument list per combination. Verified live: with no Cargo.lock the survey exits 1 with Cargo's --locked error and writes no lock; --no-locked writes it. docs/clippy.md updated.
<!-- SECTION:NOTES:END -->
