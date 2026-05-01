---
id: TASK-0150
title: 'READ-8: about subpages use println! instead of writer for output'
status: Done
assignee: []
created_date: '2026-04-22 21:22'
updated_date: '2026-04-23 15:11'
labels:
  - rust-code-review
  - READ
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Files**: `extensions/about/src/coverage.rs:68,73`, `extensions/about/src/code.rs:99`, `extensions/about/src/deps.rs:31,34`, `extensions/about/src/units.rs:32,43`

**What**: `run_about_coverage`, `run_about_code`, `run_about_deps`, `run_about_units` call `println!` directly for user-visible output, while `run_about` in `lib.rs` takes an injected `&mut dyn Write`. This inconsistency makes these subpages untestable for output content and couples them to stdout.

**Why it matters**: READ-8 flags `println!` in non-binary/non-test code. Testability and consistency with the existing `writer` injection pattern suffer; also prevents redirecting output for TAP/CI contexts.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 about subpages accept a writer (or return lines) instead of calling println!
- [ ] #2 subpages are tested against captured output
<!-- AC:END -->
