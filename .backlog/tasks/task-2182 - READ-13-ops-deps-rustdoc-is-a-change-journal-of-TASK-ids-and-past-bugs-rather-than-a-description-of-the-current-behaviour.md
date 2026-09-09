---
id: TASK-2182
title: 'READ-13: ops-deps rustdoc is a change journal of TASK ids and past bugs rather than a description of the current behaviour'
status: To Do
assignee: []
created_date: '2026-09-08 07:13'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - readability
dependencies: []
parent_task_id: 'TASK-2248'
modified_files:
  - extensions-rust/deps/src/lib.rs
  - extensions-rust/deps/src/parse/mod.rs
  - extensions-rust/deps/src/parse/upgrade.rs
  - extensions-rust/deps/src/parse/deny.rs
  - extensions-rust/deps/src/format.rs
  - extensions-rust/deps/src/test_support.rs
priority: low
ordinal: 95000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/lib.rs:35`, `extensions-rust/deps/src/parse/mod.rs:15`, `extensions-rust/deps/src/parse/upgrade.rs:144`, `extensions-rust/deps/src/parse/deny.rs:345`, `extensions-rust/deps/src/format.rs:27`, `extensions-rust/deps/src/test_support.rs:9`

**What**: nearly every doc comment in this crate narrates the review history instead of the end state. Representative sites:

- `lib.rs:35-45` — a 11-line comment on the `pub use parse::{…}` list explaining what `parse_upgrade_table` *used to be* and why it is no longer exported.
- `lib.rs:272-290` and `lib.rs:308-321` — `severity_is_actionable` / `has_issues` doc blocks that spend more lines on "previously the allowlist `matches!(s, "error" | "warning")` silently treated…" than on what the functions do.
- `parse/upgrade.rs:144-157`, `parse/upgrade.rs:357-374` — `separator_columns` doc leads with "**The invariant this does *not* rely on**" and a paragraph on the `1.10.100` decoded as `1.10.10` bug.
- `parse/deny.rs:345-358` — `resolve_package` doc is ten lines about the `&mut` / `mem::take` version that no longer exists.
- `format.rs:27-40`, `test_support.rs:9-42` — same pattern.

Counted across the crate there are roughly 60 `TASK-nnnn` references inside `///` and `//!` blocks. Sibling crates already have this filed against them (TASK-2155 for cargo-update, TASK-2169 for text-fixers, TASK-2099); ops-deps is the densest instance in the workspace.

**Why it matters**: `///` text is the published API description; a reader looking up `separator_columns` or `has_issues` has to parse three superseded designs before reaching the current contract. The history belongs in git and in the backlog tasks that are already cited by number. Rationale that is genuinely load-bearing for a future editor (e.g. "the final fixed column reads to the end of the data row because the separator run is sized to the header token") should survive as a statement about the code, not as a story about a fix.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 public and internal doc comments in ops-deps describe current behaviour and contracts; superseded designs and bug narratives are removed
- [ ] #2 rationale worth keeping is restated as a property of the present code rather than as a description of what a previous version did
- [ ] #3 TASK-nnnn references remain only where they add information a reader cannot get from the code, and never as the subject of the sentence
- [ ] #4 cargo doc still builds with no broken intra-doc links after the rewrite
<!-- AC:END -->
