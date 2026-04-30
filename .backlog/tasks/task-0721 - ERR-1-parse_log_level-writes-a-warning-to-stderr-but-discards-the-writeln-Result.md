---
id: TASK-0721
title: >-
  ERR-1: parse_log_level writes a warning to stderr but discards the writeln!
  Result
status: Done
assignee:
  - TASK-0737
created_date: '2026-04-30 05:31'
updated_date: '2026-04-30 18:07'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/main.rs:86-94`

**What**: `parse_log_level` falls back to INFO when `OPS_LOG_LEVEL` is unparseable and writes `ops: warning: invalid OPS_LOG_LEVEL...` directly to the supplied writer (`io::stderr()` in production). The `writeln!` result is bound to `let _ =`, silently discarding any I/O failure. If stderr is closed (or piped to a broken consumer) the warning vanishes without trace and the user gets the silently-defaulted INFO level — exactly the failure mode TASK-0447 (the prior tracing-swallow fix) was trying to avoid.

**Why it matters**: This is the only diagnostic the user gets when `OPS_LOG_LEVEL` is rejected. Once swallowed, the user sees neither tracing output (subscriber not yet registered, the rationale for writing direct to stderr in the first place) nor the explicit warning. Either (a) propagate the writeln error up through `parse_log_level -> init_logging -> main` so a failure to warn becomes a fatal startup error, or (b) document via comment that an unwritable stderr means the user already lost the diagnostic and that is acceptable. Today the choice is implicit.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either propagate the writeln error to the caller so init_logging surfaces it (panic, eprintln, or process exit), or add a comment that explicitly documents the chosen swallow and references this task
- [ ] #2 Test: feed parse_log_level a writer whose write returns Err and assert the chosen behaviour (propagation or documented swallow) matches the contract
<!-- AC:END -->
