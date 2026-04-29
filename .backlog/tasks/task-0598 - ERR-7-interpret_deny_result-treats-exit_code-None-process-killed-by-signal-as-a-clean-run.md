---
id: TASK-0598
title: >-
  ERR-7: interpret_deny_result treats exit_code = None (process killed by
  signal) as a clean run
status: Triage
assignee: []
created_date: '2026-04-29 05:19'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/parse.rs:263`

**What**: interpret_deny_result only errors when exit_code == Some(2); None (killed by SIGKILL/OOM-killer/parent timeout) falls through to parse_deny_output(stderr) and returns whatever partial diagnostics were flushed. Killed cargo-deny thus reports as "clean" or "fewer issues than reality".

**Why it matters**: Silent failure mode hiding supply-chain advisories. cargo-deny is the source of truth for vulnerability reporting; reporting clean when killed is a security-grade reliability bug.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 exit_code == None returns Err with clear 'deny terminated by signal' message
- [ ] #2 Test exercises the None arm against interpret_deny_result
<!-- AC:END -->
