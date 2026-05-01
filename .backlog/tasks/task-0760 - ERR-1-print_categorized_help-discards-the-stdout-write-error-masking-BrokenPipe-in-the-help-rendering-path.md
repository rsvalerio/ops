---
id: TASK-0760
title: >-
  ERR-1: print_categorized_help discards the stdout write error, masking
  BrokenPipe in the help-rendering path
status: Triage
assignee: []
created_date: '2026-05-01 05:54'
labels:
  - code-review-rust
  - errors
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/help.rs:244`

**What**: `let _ = write!(std::io::stdout(), "{out}");` — render error is silently swallowed. The same module parse_log_level (main.rs:94) at least documents why writeln to stderr can fail; here the help renderer drops the result with no comment.

**Why it matters**: A user piping `ops --help | head -5` triggers EPIPE on stdout. Mirrors TASK-0721 (parse_log_level). At minimum, document the swallow with a comment matching parse_log_level's style.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 print_categorized_help either propagates the write error (returning io::Result<()>) or carries the same "discarded — diagnostic channel is gone" comment that parse_log_level carries at main.rs:91-95
- [ ] #2 Test (e.g. via failing writer wrapper if refactored to take &mut dyn Write) pins the chosen behaviour
<!-- AC:END -->
