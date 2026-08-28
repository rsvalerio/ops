---
id: TASK-1838
title: >-
  ARCH-8: config-checkers lib.rs is the whole crate — error type, options,
  engine, and 160 lines of tests
status: To Do
assignee:
  - TASK-2004
created_date: '2026-08-27 15:22'
updated_date: '2026-08-28 14:16'
labels:
  - code-review-rust
  - structure-readability
dependencies: []
modified_files:
  - extensions/config-checkers/src/lib.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/config-checkers/src/lib.rs` (485 lines)

**What**: `lib.rs` is supposed to be the thin entry point — module declarations, re-exports, crate docs, small central types. This one is the crate. Its inventory:

| Lines | Content |
|-------|---------|
| 44-72 | extension registration (`impl_extension!`) |
| 74-102 | `CheckError` + hand-written `Display` + `Error::source` |
| 104-140 | `CheckerOptions` + three builder methods |
| 142-165 | `FailedFile`, `CheckerReport`, `CheckerReport::failed` |
| 172-204 | `run_check_json` / `run_check_yaml` |
| 206-209 | `matches_ext` |
| 211-297 | `run_checker` — the entire checking engine, 87 lines |
| 299-301 | `relative_to` |
| 303-320 | `write_summary` |
| 322-485 | `mod tests` — 164 lines |

Meanwhile the two modules that exist, `json.rs` and `yaml.rs`, hold **11 and 6 lines of production code respectively**. The split is inverted: the trivially-separable parser wrappers got their own files, and everything with actual structure stayed in `lib.rs`.

ARCH-8's own heuristic is met twice over: "inline error types for <=2 types in <=50 lines; when types exceed ~50 lines or you have 3+ helper functions, extract to a dedicated module". Here the domain types run 74-165 (~90 lines) and there are four non-public helpers (`matches_ext`, `run_checker`, `relative_to`, plus `write_summary` as public output logic). `CheckError` is additionally *shared across modules* — both `json.rs` and `yaml.rs` import it — which is ARCH-8's explicit trigger for an `error.rs`.

**Why it matters**: ARCH-8, with the sibling crates as the calibration rather than an abstract rule. Every comparable extension in this workspace already splits this way and this one is the outlier:

```
extensions/text-fixers/src/lib.rs        246   (+ discovery.rs, binary.rs, eof.rs, trailing.rs)
extensions-rust/loc/src/lib.rs           222   (+ counter.rs, ingestor.rs, views.rs, tests.rs)
extensions-rust/cargo-toml/src/lib.rs    300   (+ types.rs, inheritance.rs, workspace_root.rs, tests.rs)
extensions-rust/deps/src/lib.rs          340   (+ types.rs, format.rs, parse/, tests.rs)
extensions/config-checkers/src/lib.rs    485   (+ json.rs 11 LoC, yaml.rs 6 LoC)
```

`extensions/text-fixers` is the closest analogue — same shape of problem, same discovery walk, which this crate reuses — and it puts its engine in named modules and is half the size. Two neighbours (`loc`, `cargo-toml`) also carry their test module as `tests.rs`, which is the existing answer to the 164 inline test lines here.

This is Low severity and mostly dormant, but it compounds with the other findings against this file: TASK-1815 (`run_checker` at 87 lines with three duplicated record-and-report blocks), TASK-1813 (`FailedFile`/`CheckerReport` need a typed failure kind) and TASK-1824 (`CheckError::Parse` should carry a boxed source) all edit `lib.rs`, and all three land more cleanly once each concern has a file.

**Fix shape**: follow the neighbours. `error.rs` for `CheckError` + its `Display`/`Error` impls (shared by both checker modules, so it belongs there by ARCH-8's stated rule); `report.rs` (or `options.rs` + `report.rs`) for `CheckerOptions`, `FailedFile`, `CheckerReport`, and `write_summary`; `runner.rs` for `run_checker`, `matches_ext`, `relative_to`, and the two `run_check_*` entry points; `tests.rs` for the test module, matching `loc` and `cargo-toml`. `lib.rs` keeps the crate docs, `mod` declarations, curated `pub use` re-exports so the public paths are unchanged, the `NAME`/`DESCRIPTION`/`SHORTNAME`/`DEFAULT_MAX_BYTES` constants, and the `impl_extension!` block. Best sequenced with or after TASK-1815, since that finding is already refactoring the engine's body.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 CheckError and its Display/Error impls live in their own module, since both json.rs and yaml.rs depend on them
- [ ] #2 The checker engine (run_checker, matches_ext, relative_to, run_check_json, run_check_yaml) and the report/options types are moved out of lib.rs into named modules
- [ ] #3 lib.rs is reduced to crate docs, module declarations, re-exports, the crate constants, and the impl_extension! block, in line with the sibling extension crates
- [ ] #4 The crate's public API paths are unchanged (re-exported from lib.rs) and all existing tests still pass
<!-- AC:END -->
