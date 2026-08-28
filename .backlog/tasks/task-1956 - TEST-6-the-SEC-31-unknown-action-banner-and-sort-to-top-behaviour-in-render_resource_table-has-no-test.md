---
id: TASK-1956
title: >-
  TEST-6: the SEC-31 unknown-action banner and sort-to-top behaviour in
  render_resource_table has no test
status: To Do
assignee:
  - TASK-2002
created_date: '2026-08-27 15:50'
updated_date: '2026-08-28 14:15'
labels:
  - code-review-rust
  - testing
dependencies: []
modified_files:
  - extensions-terraform/plan/src/render.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/plan/src/render.rs:98-121`, tests at `:195-331`

**What**: `Action::Unknown` never appears in `render.rs`'s test module. The whole SEC-31 / TASK-0833 mechanism on the render side is unexercised:

- the `unknown_count > 0` branch and the WARNING banner string at `:106-113`
- `sort_priority() == 0` putting Unknown rows above Delete at `:115-121` (the existing `resource_table_sorted_delete_first` at `:244` only compares delete/create/update)
- `ACTION_DISPLAY_ORDER` listing Unknown first in the summary table at `:19-27` and `:51-57`

The unit coverage that does exist for Unknown lives in `model.rs:179-194` and `lib.rs:413-426`, and stops at classification - it proves an unrecognized action becomes `Action::Unknown`, never that the operator actually sees it.

Also untested in this module: `render_resource_table`'s `is_tty=true` width-capping branch at `:133-137` and `:150-156` (`MODULE_COL_MIN_WIDTH` / `NON_MODULE_COLS_RESERVED` are pure arithmetic that could be checked directly), and the `module` column at all - every `make_change` helper at `:199-208` sets `module: None`, so `c.module.as_deref().unwrap_or("")` at `:141` has only ever been exercised on the `None` side.

**Why it matters**: TEST-6 asks for error paths and edge cases, not just happy paths. The banner exists specifically so an operator does not miss audit-relevant rows; a regression that drops it (an inverted condition, a renamed label) would pass the whole suite today.

**Note**: filed as TEST-6 rather than the SEC-31 rule already used for `render_outputs_table` in this file - this is the separate, test-coverage-shaped defect on the resource-table side.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A test renders a change set containing an Unknown action and asserts the WARNING banner text and the count it reports
- [ ] #2 A test asserts an Unknown row sorts above a Delete row
- [ ] #3 A test asserts the summary table lists unknown before the other actions
- [ ] #4 A test covers a change with Some(module) so the module column is exercised on both sides of the Option
<!-- AC:END -->
