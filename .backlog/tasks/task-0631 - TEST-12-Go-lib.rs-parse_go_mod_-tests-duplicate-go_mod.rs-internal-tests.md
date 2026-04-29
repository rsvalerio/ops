---
id: TASK-0631
title: 'TEST-12: Go lib.rs parse_go_mod_* tests duplicate go_mod.rs internal tests'
status: Done
assignee:
  - TASK-0641
created_date: '2026-04-29 05:22'
updated_date: '2026-04-29 12:12'
labels:
  - code-review-rust
  - TEST
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/lib.rs:127`

**What**: extensions-go/about/src/lib.rs:127-461 contains a mod tests whose parse_go_mod_basic, parse_go_mod_local_replaces, parse_go_mod_no_go_version, parse_go_mod_missing, parse_go_work_* tests cover the same parser surface that extensions-go/about/src/go_mod.rs:80-139 and go_work.rs:63-86 already test. lib.rs is testing the parse_go_mod re-wrapper which is a thin destructure of go_mod::parse.

**Why it matters**: Test-suite duplication slows iteration loop and gives false confidence in coverage when a parser change has to be reflected in two test files.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Pure-parser tests in lib.rs (parse_go_mod_basic, parse_go_mod_no_go_version, parse_go_mod_local_replaces, parse_go_work_multi_use, parse_go_work_single_use, parse_go_work_missing, parse_go_mod_ignores_remote_replaces, parse_go_mod_whitespace_handling, parse_go_mod_empty_file, parse_go_mod_no_module_line, parse_go_work_empty_use_block, parse_go_work_comments_in_use_block, parse_go_work_empty_file, parse_go_work_blank_lines_in_use_block, parse_go_work_multiple_single_line_uses) move into go_mod.rs / go_work.rs (or are removed where covered)
- [ ] #2 provide_* integration tests stay in lib.rs
- [ ] #3 cargo test -p ops-about-go passes
<!-- AC:END -->
