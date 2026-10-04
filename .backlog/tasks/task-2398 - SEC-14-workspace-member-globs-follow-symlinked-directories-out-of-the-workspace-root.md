---
id: TASK-2398
title: 'SEC-14: workspace member globs follow symlinked directories out of the workspace root'
status: To Do
assignee: []
created_date: '2026-10-04 14:15'
updated_date: '2026-10-04 14:52'
labels:
  - code-review-rust
  - SEC
dependencies: []
parent_task_id: 'TASK-2421'
modified_files:
  - extensions/about/src/workspace.rs
priority: low
ordinal: 1000
dedup_key: 'SEC-14:extensions/about/src/workspace.rs:glob_child_dir'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/workspace.rs:glob_child_dir`, `extensions/about/src/workspace.rs:resolve_member_globs` (Literal arm, `root.join(literal)`)

**What**: `member_escape` rejects `..` and absolute member strings, but `glob_child_dir` accepts any entry where `path.is_dir()` is true, which follows symlinks, and the `Literal` arm joins the member straight onto the root. A repo containing `packages/x -> /some/other/dir` (checked in as a symlink) makes `ops about` read `<marker>` (package.json / pyproject.toml) from outside the root and surface its name/version/description in the rendered output. `is_dir()` then a separate open is also a check-then-use pair (SEC-25).

**Why it matters**: The module's own doc states the invariant "members cannot escape root"; symlinks break it for adversarial repos. Impact is bounded (manifest fields only), hence low.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Symlinked member directories are skipped (use DirEntry::file_type, not is_dir) or verified to canonicalize inside the root
- [ ] #2 The Literal member path gets the same containment check
- [ ] #3 Regression test with a symlinked member pointing outside the root
<!-- AC:END -->
