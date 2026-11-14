---
id: TASK-2661
title: 'atomic_write''s staging name overflows NAME_MAX for task files named past ~242 bytes, so task edit fails with File name too long'
status: In Progress
assignee: []
created_date: '2026-11-14 11:23'
updated_date: '2026-11-14 15:21'
labels:
  - backlog
  - bug
dependencies: []
modified_files:
  - crates/backlog/src/cmd/mod.rs
priority: medium
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/backlog/src/cmd/mod.rs:129` (`stage_contents`, called by `atomic_write`)

**What**: the staging file is named `.{name}.{pid}.tmp` next to the destination, which adds about 13 bytes (the dot, `.<pid>` and `.tmp`) to the task file name. A task file name of more than about 242 bytes then produces a staging name past the 255-byte NAME_MAX, so `ops backlog task edit` fails with `staging … File name too long (os error 36)` and writes nothing. `task create` writes the long name fine, so the tool creates tasks it then cannot edit.

**Seen in**: dbsec code-review run 2026-11-14. TASK-1895 (249-byte file name), TASK-1896 (244), TASK-1882 and TASK-1900 (251) could not be edited. Wave runners fell back to hand-editing frontmatter or shortening titles. dbsec tracks the consumer side as TASK-1937, and 10 of its task files are still past the limit. Review findings have long, descriptive titles, so this keeps recurring.

**Fix options**: derive the staging name from a bounded prefix or hash of the destination name (e.g. `.{task-id}.{pid}.tmp`, or truncate `name` so the whole staging name fits in NAME_MAX), and/or cap the slug that `task create` derives from the title so the file name leaves room for staging.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 ops backlog task edit succeeds on a task whose file name is 255 bytes long
- [x] #2 a regression test edits a task whose file name is at the NAME_MAX limit

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Fixed in crates/backlog/src/cmd/mod.rs: staging_name() keeps .{name}.{pid}.tmp when it fits NAME_MAX (255), otherwise truncates the name on a char boundary and appends a 16-hex DefaultHasher digest of the full name so shared prefixes stay distinct. Covers atomic_write and atomic_write_noclobber (both stage via stage_contents). Verified: unit tests for staging_name, edit.rs regression test edit_succeeds_when_the_file_name_is_at_name_max, end-to-end ops backlog task edit on a 255-byte file name in a temp repo, ops verify 9/9 and ops qa 7/7. Change uncommitted.
<!-- SECTION:NOTES:END -->
