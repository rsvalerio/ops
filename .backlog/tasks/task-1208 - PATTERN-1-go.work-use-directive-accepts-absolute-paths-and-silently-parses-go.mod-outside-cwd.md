---
id: TASK-1208
title: >-
  PATTERN-1: go.work use directive accepts absolute paths and silently parses
  go.mod outside cwd
status: To Do
assignee:
  - TASK-1270
created_date: '2026-05-08 08:17'
updated_date: '2026-05-08 13:19'
labels:
  - code-review-rust
  - pattern
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/modules.rs:67-91`

**What**: unit_from_use_dir calls normalize_module_path(dir) which only strips a leading ./ / .\\ and trailing slashes; an entry like `use /etc/secrets` survives normalisation as /etc/secrets, then cwd.join(&normalized) returns the absolute path verbatim, and read_mod_info opens whatever go.mod-shaped file lives there. The out-of-tree warn fires only when the first path component is `..`.

**Why it matters**: Operator-controlled config so impact is bounded, but the same threat-model that motivated SEC-14 / TASK-1071 in workspace.rs::resolve_member_globs applies here. Inconsistent treatment with the Node/Python resolve_member_globs traversal guard makes this an easy vector to forget.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 unit_from_use_dir rejects (or marks out-of-tree) any normalized path whose first Path::components() is Component::RootDir / Prefix / ParentDir, matching the resolve_member_globs guard.
- [ ] #2 A new test collect_units_absolute_use_directive_is_marked_out_of_tree writes a go.work containing 'use /etc' and asserts read_mod_info is not invoked against the absolute target, with one tracing::warn on the directive.
<!-- AC:END -->
