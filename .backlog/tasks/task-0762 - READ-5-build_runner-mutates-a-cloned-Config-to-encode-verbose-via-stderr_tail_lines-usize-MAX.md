---
id: TASK-0762
title: >-
  READ-5: build_runner mutates a cloned Config to encode "verbose" via
  stderr_tail_lines = usize::MAX
status: Triage
assignee: []
created_date: '2026-05-01 05:54'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/run_cmd.rs:74-76`

**What**: `if verbose { config.output.stderr_tail_lines = usize::MAX; }` overloads a numeric tail-lines field with a sentinel meaning "unbounded". Verbose mode is a Boolean run-mode setting; threading it through a numeric field obscures intent.

**Why it matters**: A future config change that lowers stderr_tail_lines defaults will appear to "work" in verbose mode (which silently overrides) but break in normal mode. Conversely a user who set stderr_tail_lines = 100_000 and runs --verbose cannot tell whether their setting was honoured.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 ProgressDisplay/runner takes a typed StderrTail::Unbounded | Limited(usize) (or the verbose flag directly) instead of decoding usize::MAX as a sentinel
- [ ] #2 Config field is no longer mutated post-load; runner reads user setting verbatim and applies verbose override at the display layer
- [ ] #3 Test pins that a user-configured stderr_tail_lines = 1000 is preserved in the threaded Config even when verbose is true
<!-- AC:END -->
