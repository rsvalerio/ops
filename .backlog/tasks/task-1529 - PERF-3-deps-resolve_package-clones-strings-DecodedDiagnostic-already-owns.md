---
id: TASK-1529
title: 'PERF-3: deps resolve_package clones strings DecodedDiagnostic already owns'
status: Done
assignee:
  - TASK-1647
created_date: '2026-05-19 07:33'
updated_date: '2026-05-25 18:53'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/parse/deny.rs:234-255`

**What**: `resolve_package` does `a.package.clone()` and `k.name.clone()` even though the chosen branch is consumed once by `push_diagnostic`. With `decode_diagnostic` already returning an owned `DecodedDiagnostic`, the function could take ownership and `take()` / move the field instead of cloning.

**Why it matters**: Per-diagnostic String allocation on the hot path; trivially avoidable by passing the diagnostic by value into `resolve_package`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Change signature to take ownership (or pass relevant subfields by value)
- [ ] #2 Use .take() on Option<String> to move the value out without cloning
- [ ] #3 parse_deny_* tests pass
<!-- AC:END -->
