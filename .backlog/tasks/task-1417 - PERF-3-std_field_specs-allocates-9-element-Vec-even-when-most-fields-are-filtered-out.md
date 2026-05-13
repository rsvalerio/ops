---
id: TASK-1417
title: >-
  PERF-3: std_field_specs allocates 9-element Vec even when most fields are
  filtered out
status: Done
assignee:
  - TASK-1452
created_date: '2026-05-13 18:17'
updated_date: '2026-05-13 20:35'
labels:
  - code-review-rust
  - perf
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/project_identity/card.rs:22`

**What**: `std_field_specs` unconditionally constructs a Vec of nine `(id, label, Option<value>)` tuples — including `format!` for `dependencies` and `compose_stack_value`/`compose_project_value`/`compose_codebase_value` String allocations — and then `from_identity_filtered` filters most of them out via the `show` closure when `visible_fields` pins a small subset.

**Why it matters**: The about card is on the default `ops` invocation. When `visible_fields = Some(&["project"])` we still compute stack/codebase/dependencies/repository/homepage values and discard them. Push the filter into spec generation (or convert to per-field-id match computing values only for shown fields).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 skip computing field values for ids not in visible_fields
- [ ] #2 preserve all-fields-shown behaviour when visible_fields is None
- [ ] #3 regression test pins that filtered card rendering does not invoke compose_codebase_value when only 'project' is requested
<!-- AC:END -->
