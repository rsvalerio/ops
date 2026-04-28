---
id: TASK-0526
title: 'ERR-1: is_in_cargo_list strips ''cargo-'' prefix without rejecting empty result'
status: To Do
assignee:
  - TASK-0533
created_date: '2026-04-28 06:52'
updated_date: '2026-04-28 07:14'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/probe.rs:57`

**What**: is_in_cargo_list strips a leading `cargo-` from the name. If a caller asks for the literal name `cargo-`, the strip yields the empty string, which matches lines with leading whitespace as the first whitespace token is empty.

**Why it matters**: Defensive — current callers pass real tool names, but validate_cargo_tool_arg does not reject `cargo-`. A malformed [tools] entry could trigger a false positive.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Reject empty-after-strip names early
- [ ] #2 Unit test for cargo- and the empty string
<!-- AC:END -->
