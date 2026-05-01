---
id: TASK-0799
title: >-
  ERR-7: interpret_deny_result lets every cargo-deny exit code other than 0/1/2
  fall through to parse_deny_output
status: Triage
assignee: []
created_date: '2026-05-01 06:01'
labels:
  - code-review-rust
  - errors
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/parse.rs:270-300`

**What**: The function explicitly handles Some(2) (configuration error) and None (signal). It also rejects empty stderr at Some(1). Anything else — Some(3), Some(101) (cargo-deny panic), a future code, a negative POSIX-mapped status — falls through to Ok(parse_deny_output(stderr)), which for non-diagnostic stderr returns an empty DenyResult and reports a clean run.

**Why it matters**: Mirrors TASK-0598/TASK-0612. cargo-deny panics today still produce 101. A future supply-chain gate change should fail-closed.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Add an explicit fail-closed branch for Some(code) outside the documented {0, 1} contract: bail with cargo deny exited with unexpected status code
- [ ] #2 Update the doc comment to spell out the explicit codes accepted (0 clean, 1 issues, 2 config) and the fail-closed default for everything else
- [ ] #3 Add a unit test asserting interpret_deny_result(Some(101), panicked-stderr) returns Err
<!-- AC:END -->
