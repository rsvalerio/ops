---
id: TASK-2685
title: 'SEC-11: task edit --parent stores any string verbatim, unlike create --parent'
status: Done
assignee: []
created_date: '2026-11-18 18:27'
updated_date: '2026-11-18 21:26'
labels:
  - code-review-rust
  - SEC
dependencies: []
parent_task_id: 'TASK-2837'
modified_files:
  - crates/backlog/src/cmd/edit.rs
priority: medium
ordinal: 1000
dedup_key: 'SEC-11:crates/backlog/src/cmd/edit.rs:apply_frontmatter_edits'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/backlog/src/cmd/edit.rs:93-97` (`apply_frontmatter_edits`)

**What**: `create_task` resolves `--parent` through `Store::find`, refuses an unknown parent, and stores the parent's canonical id (`create.rs:102-112`). `task edit --parent` does neither: it writes whatever string it was given into `parent_task_id`. Reproduced on ops 0.94.1: `ops backlog task edit 1 --parent 99` succeeds on a backlog with no task 99 and writes `parent_task_id: '99'`.

Consumers compare `parent_task_id` against canonical ids with `eq_ignore_ascii_case` (`wave.rs:175-177` `is_member`, `list.rs:80-88` `parent_matches`, `wave_create.rs:147`), so a bare (`1764`) or differently padded parent written via edit never matches its wave, and a typo'd parent is accepted silently.

**Why it matters**: Wave membership and `task list --parent` silently drop members linked through `edit`, the path the wave skills use to re-parent findings.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 task edit --parent resolves the id through Store::find, errors when it resolves to nothing, and stores the canonical id — the same contract as create --parent
- [x] #2 A test edits --parent with a bare number and with an unknown id and asserts the canonical id is stored / an error is returned

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.01 (code-review-rust run, 2026-11-18).

Fixed in TASK-2837: edit --parent resolves through Store::find, errors on unknown, stores canonical id. Test: parent_is_resolved_to_its_canonical_id_or_refused.

<!-- SECTION:NOTES:END -->
