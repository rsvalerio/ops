---
id: TASK-0772
title: >-
  PERF-2: build_command_with collects expanded args into a fresh Vec<String>
  instead of streaming into Command::args
status: Triage
assignee: []
created_date: '2026-05-01 05:56'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/build.rs:292-297`

**What**: expanded_args: Vec<String> is built then passed to cmd.args(&expanded_args). Each arg is into_owned() first, allocating a String per arg, then collected into a Vec, then iterated again by Command::args.

**Why it matters**: Per-spawn, on the parallel hot path. cmd.arg accepts &OsStr and can be called in a loop directly with the Cow<'_, str> from try_expand without materialising the Vec or owned Strings (Cow::Borrowed → no allocation when expansion is a no-op, the common case).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace the collect-into-Vec<String> with a for loop calling cmd.arg(expanded.as_ref()) and propagating ExpandError via ?
- [ ] #2 Keep the early-return semantics on the first failing arg
- [ ] #3 Confirm with a microbench or trace that the no-op expansion path drops to zero arg-string allocations
<!-- AC:END -->
