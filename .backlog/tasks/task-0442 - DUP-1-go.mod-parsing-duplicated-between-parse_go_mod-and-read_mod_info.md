---
id: TASK-0442
title: 'DUP-1: go.mod parsing duplicated between parse_go_mod and read_mod_info'
status: Done
assignee:
  - TASK-0532
created_date: '2026-04-28 04:44'
updated_date: '2026-04-28 15:44'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Files**:
- `extensions-go/about/src/lib.rs:102-128` (parse_go_mod)
- `extensions-go/about/src/modules.rs:86-111` (read_mod_info)

**What**: Both functions reimplement the "open go.mod, iterate trimmed lines, capture `module ...` and `go ...`" loop. The recent refactor (commit 1785138) extracted go_work::parse_use_dirs to a shared module but did not extract the analogous go.mod parser. parse_go_mod additionally captures local_replaces, but the common subset is byte-identical logic.

**Why it matters**: Sibling to TASK-0388 (go.work duplication, Done). Future fixes (UTF-8 BOM, // comments inside go.mod, the tab-indented form `\tgo 1.x` inside a `go ( ... )` block) must be applied twice; one was already missed (read_mod_info does not call for_each_trimmed_line and so handles I/O errors via inline match while parse_go_mod uses the shared helper).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A single go_mod::parse(...) returns module + go_version + (optionally) local replaces from one place
- [ ] #2 parse_go_mod and read_mod_info consume that shared parser; behavior on existing tests is unchanged
- [ ] #3 I/O error handling in both call sites flows through the same diagnostic path (consistent tracing::debug!)
<!-- AC:END -->
