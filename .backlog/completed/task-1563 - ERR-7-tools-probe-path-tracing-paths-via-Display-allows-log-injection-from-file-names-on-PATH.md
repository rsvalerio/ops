---
id: TASK-1563
title: >-
  ERR-7: tools::probe::path tracing paths via Display allows log injection from
  file names on PATH
status: Done
assignee:
  - TASK-1578
created_date: '2026-05-19 15:56'
updated_date: '2026-05-19 18:48'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/probe/path.rs:46`, `extensions-rust/tools/src/probe/path.rs:114`

**What**: `capture_path_index_from` logs `path = %dir.display()` for unreadable PATH entries and `find_on_path_in` logs `path = %candidate.display()` for broken symlinks. A PATH directory or file name containing newlines / ANSI escapes (legal on Unix) flows directly into tracing fields via Display.

**Why it matters**: Same log-forgery vector pinned by TASK-0965 / TASK-0974 / TASK-0999 / TASK-0979 in other modules: log scrapers can be misled by injected `\n` or escape sequences. Operator-controlled rather than attacker-supplied in practice, hence Low, but the codebase has already standardised on `?`-format for all path/error fields.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Both tracing::warn! callsites in probe/path.rs render path fields via the ? formatter (or wrap in format_log_safe) so embedded \n/\u{1b} are escaped
- [ ] #2 Regression test: a temp PATH directory whose name contains \n and \u{1b} does not produce multi-line or escape-bearing tracing output
- [ ] #3 No behavioural change to PATH walking itself
<!-- AC:END -->
