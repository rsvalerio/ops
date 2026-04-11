---
id: TASK-016
title: "Secret detection predicates use complex boolean chains"
status: To Do
assignee: []
created_date: '2026-04-07 12:00:00'
labels: [rust-code-quality, CQ, FN-5, low, effort-S, crate-runner]
dependencies: []
---

## Description

**Location**: `crates/runner/src/command/exec.rs:151-195`
**Anchor**: `fn has_high_entropy`, `fn looks_like_uuid`
**Impact**: `has_high_entropy` (line 167) chains 4 conditions (`alphanumeric > 15 && digits > 3 && lowercase > 3 && uppercase > 3`). `looks_like_uuid` (lines 186-194) chains 7 conditions for UUID format validation. Both exceed the 3-condition threshold for named predicates.

**Notes**:
These are well-isolated predicate functions (already extracted per CQ-005), so the cognitive load is contained. Possible improvements:
- `has_high_entropy`: extract a named predicate `let has_mixed_chars = digits > 3 && lowercase > 3 && uppercase > 3;` to clarify intent.
- `looks_like_uuid`: extract `fn has_uuid_segment_lengths(parts: &[&str]) -> bool` or use a const array `[8, 4, 4, 4, 12]` with `zip` + `all` to make the format spec data-driven.
Severity is low because the functions are already well-named and tested.
