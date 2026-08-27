---
id: TASK-1954
title: >-
  SEC-31: render_outputs_table fails open to no-op when an output's actions
  field is missing or malformed
status: Triage
assignee: []
created_date: '2026-08-27 15:49'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions-terraform/plan/src/render.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/plan/src/render.rs:176-190`

**What**:

    let actions = value
        .get("actions")
        .and_then(|a| a.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect::<Vec<_>>())
        .unwrap_or_default();

    let action = Action::classify(&actions).unwrap_or(Action::NoOp);

Three separate degradations all collapse to the same benign-looking "no-op" row:

1. `actions` key absent - `and_then` yields `None`, so `unwrap_or_default()` gives an empty vec
2. `actions` present but not an array - same
3. `actions` an array whose elements are not strings (numbers, objects, nulls) - `filter_map` drops them silently, and an array of only non-strings also produces an empty vec

`Action::classify(&[])` returns `None` by contract (`model.rs:51`), and the `unwrap_or(Action::NoOp)` then *labels the row as no-op*. So an output whose planned change this build cannot read renders as "nothing is happening to this output".

This directly contradicts the policy the same enum enforces for resources. `Action::classify` was deliberately changed under SEC-31 / TASK-0833 to return `Some(Action::Unknown)` with a `tracing::warn!` for unrecognized sequences (`model.rs:60-66`), and `render_resource_table` prepends a WARNING banner for them (`:106-113`). The outputs table opts out of both: no `Unknown`, no banner, no warning log.

**Why it matters**: SEC-31 - fail closed, no security bypass on error. Terraform outputs are frequently the sensitive surface of a stack (generated credentials, endpoints). Displaying an unreadable output change as "no-op" on the screen an operator reads before approving an apply is a silent fail-open.

**Suggested fix**: map the degraded cases to `Action::Unknown` (not `NoOp`), emit the same `tracing::warn!`, and extend the WARNING banner logic to the outputs table.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 An output whose actions key is missing, non-array, or contains non-string elements renders as unknown rather than no-op
- [ ] #2 The degraded case emits a tracing warning naming the output
- [ ] #3 The unrecognized-action banner is shown for the outputs table on the same terms as the resource table
- [ ] #4 Tests cover a missing actions key, a non-array actions value, and an actions array of non-strings
<!-- AC:END -->
