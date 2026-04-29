---
id: TASK-0612
title: >-
  ERR-1: interpret_deny_result treats exit code 1 with empty stderr as "issues
  parsed"
status: Done
assignee:
  - TASK-0638
created_date: '2026-04-29 05:20'
updated_date: '2026-04-29 10:43'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/parse.rs:263`

**What**: cargo-deny exit 1 with empty stderr (e.g. binary crashed before printing JSON stream) is not distinguishable from "no diagnostics emitted". Returns Ok(DenyResult::default()) and has_issues returns false — `ops deps` exits 0 when the deny pipeline silently failed. Companion to TASK-0386 but the empty-stderr-with-exit-1 path is not exercised.

**Why it matters**: Same supply-chain reliability concern as the exit_code=None finding. cargo-deny contract is "exit 1 implies stderr has the diagnostic stream".
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Exit code 1 with empty/whitespace-only stderr returns Err with clear diagnostic
- [ ] #2 Test exercises that path
<!-- AC:END -->
