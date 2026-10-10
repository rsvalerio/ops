---
id: TASK-2579
title: 'FN-1: run_fixer in ops-text-fixers spans 103 code lines, mixing orchestration with per-file outcome handling'
status: To Do
assignee: []
created_date: '2026-10-10 15:45'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - fn
dependencies: []
parent_task_id: 'TASK-2627'
modified_files:
  - extensions/text-fixers/src/runner.rs
priority: medium
ordinal: 1000
dedup_key: 'FN-1:extensions/text-fixers/src/runner.rs:run_fixer'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/text-fixers/src/runner.rs:75-202`

**What**: `run_fixer` measures 103 non-comment, non-blank lines (128 raw), far past the 50-line threshold. The single function body mixes three abstraction levels: run-level orchestration (discovery, fallback notice, walk-error accounting), per-file outcome dispatch (skipped / failed / unchanged / check-mode / write-back, each with its own reporting and counter arithmetic), and line-level writer formatting. The per-file loop body (runner.rs:126-199) is a self-contained pipeline step that can be extracted as a named helper returning one outcome enum.

**Why it matters**: FN-1 flags outliers for review because long functions mix abstraction levels and make the per-file accounting contract (scanned + skipped + failed must equal the discovered total — pinned by `every_discovered_file_is_accounted_for` and `a_file_whose_write_fails_is_counted_once`) harder to verify at a glance. The deferred-scan-tally subtlety at runner.rs:178-195 is exactly the kind of invariant that extraction into a per-file handler with documented outcomes would make structural.

**Rule**: FN-1 (Functions & structure) — functions <=50 lines; context may justify exceptions, but here the loop body has a natural extraction boundary that does not exist for state-machine-style matches.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 run_fixer's body is reduced to orchestration (discovery, notices, the candidate loop) with per-file handling extracted into one or more named helpers
- [ ] #2 run_fixer and each extracted helper are at or under 50 code lines (excluding comments and blanks)
- [ ] #3 The scanned + skipped + failed accounting is unchanged; all existing tests in extensions/text-fixers pass without modification
<!-- AC:END -->
