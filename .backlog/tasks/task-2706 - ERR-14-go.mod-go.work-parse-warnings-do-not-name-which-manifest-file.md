---
id: TASK-2706
title: 'ERR-14: go.mod/go.work parse warnings do not name which manifest file'
status: Done
assignee: []
created_date: '2026-11-18 18:32'
updated_date: '2026-11-18 22:20'
labels:
  - code-review-rust
  - ERR
dependencies: []
parent_task_id: 'TASK-2846'
modified_files:
  - extensions-go/about/src/go_mod.rs
  - extensions-go/about/src/go_work.rs
priority: medium
ordinal: 1000
dedup_key: 'review-2667.07-1'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/go_mod.rs:119`, `extensions-go/about/src/go_work.rs:63`, `extensions-go/about/src/go_work.rs:87`

**What**: The unterminated-block warning in `go_mod::parse` (go_mod.rs:119-123) carries only `manifest = "go.mod"` and the directive name — not the path of the file that failed. `go_mod::parse` runs once for the root and once per `go.work` `use` directory (`modules::read_mod_info` -> `unit_from_use_dir`), so in a workspace the operator gets "go.mod: unterminated block directive" with no way to tell which of N `go.mod` files is malformed. The `go.work` warnings (nested opener at go_work.rs:63, unterminated block at go_work.rs:87) likewise omit the path and the line number of the offending opener.

**Why it matters**: These diagnostics exist precisely so a malformed hand-edited manifest does not silently look like a missing one (crate docs, lib.rs:8-10). A warning that cannot be located is unactionable in a multi-module workspace. Same spirit as ERR-13/ERR-14: diagnostics on human-edited input must point at where the problem is.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 go_mod::parse warnings include the manifest path as a structured Debug-formatted field (e.g. path = ?path)
- [x] #2 go_work::parse_use_dirs warnings include the go.work path and the 1-based line number of the offending/unterminated opener
- [x] #3 A test asserts the rendered warn for a nested use-dir go.mod names that subdirectory's path

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.07 (code-review-rust run, 2026-11-18).

go_mod::parse_content and go_work::parse_use_dirs_content warn with path = ?path; go.work warns also carry line_number (nested opener line / unterminated opener line). modules::tests::nested_use_dir_go_mod_warn_names_its_path asserts the api/go.mod path.

<!-- SECTION:NOTES:END -->
