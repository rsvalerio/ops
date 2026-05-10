---
id: TASK-0601
title: 'ERR-2: has_issues ignores any cargo-deny severity outside error|warning'
status: Done
assignee:
  - TASK-0639
created_date: '2026-04-29 05:19'
updated_date: '2026-04-29 11:00'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/lib.rs:192`

**What**: is_actionable hard-codes `matches!(s, "error" | "warning")`. Any cargo-deny severity outside that pair (help, note, future critical) is treated as non-actionable. Combined with parse_deny_output severity.unwrap_or("error"), an unknown severity that was explicitly emitted gets treated as benign while a missing severity is treated as error — backwards.

**Why it matters**: A schema drift in cargo-deny severity vocabulary would silently let new diagnostic classes pass `ops deps` without failing the command. The fail-on-issues gate is what makes the command useful in CI.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Either fail-closed (any severity not explicitly matched fails) or fail-open with warn log naming unknown severity
- [x] #2 Test covers an injected unknown severity
<!-- AC:END -->
