---
id: TASK-2716
title: 'TEST-33: go.mod/go.work parsers are fused to filesystem reads with no &str entry point'
status: Done
assignee: []
created_date: '2026-11-18 18:32'
updated_date: '2026-11-18 22:20'
labels:
  - code-review-rust
  - TEST
dependencies: []
parent_task_id: 'TASK-2846'
modified_files:
  - extensions-go/about/src/go_mod.rs
  - extensions-go/about/src/go_work.rs
priority: low
ordinal: 1000
dedup_key: 'review-2667.07-6'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/go_mod.rs:46`, `extensions-go/about/src/go_work.rs:26`

**What**: `go_mod::parse(dir: &Path)` and `go_work::parse_use_dirs(root: &Path)` read the file via `ops_about::manifest_io::read_optional_text` and parse inline in the same function. There is no `parse_str(&str)`-shaped entry point, so every one of the ~35 parser unit tests (go_mod.rs, go_work.rs) creates a `tempfile::tempdir()` and writes a fixture to disk just to exercise pure line grammar.

**Why it matters**: The grammar (comment stripping, block openers/terminators, unterminated-block rollback) is pure string logic and the crate's highest-risk code; coupling it to disk makes each case slower and noisier and rules out cheap property tests (TEST-9: "arbitrary input never panics / unterminated block never leaks entries") over the parser.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Each parser is split into a thin I/O wrapper and a pure fn over &str (e.g. parse_content / parse_use_dirs_content)
- [x] #2 Grammar-only unit tests call the pure fn without tempdir; tests that exercise missing/unreadable files keep using tempdir
- [x] #3 Public behaviour of parse / parse_use_dirs is unchanged

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.07 (code-review-rust run, 2026-11-18).

Pure fns go_mod::parse_content(&str, &Path) and go_work::parse_use_dirs_content(&str, &Path); the Path is diagnostics-only. Grammar tests call them via parse_str; missing-file and one happy-path test per parser stay on tempdir.

<!-- SECTION:NOTES:END -->
