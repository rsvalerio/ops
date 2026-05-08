---
id: TASK-1071
title: 'PATTERN-1: about workspace member values not rejected for ../ traversal'
status: Done
assignee: []
created_date: '2026-05-07 21:19'
updated_date: '2026-05-07 23:34'
labels:
  - code-review-rust
  - PATTERN
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/workspace.rs:88`

**What**: Non-glob workspace member values are joined to `root` without any `..` rejection. A `.ops.toml` (or upstream manifest) member entry of `"../sibling"` resolves and loads a manifest outside the workspace.

**Why it matters**: Workspace config is operator-controlled so the impact is low, but `try_read_manifest(&root.join(member), marker)` is the only surface where a path-traversal `member` reaches I/O. Aligns with the SEC-13 dot-only-segment work in `git/src/remote.rs`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Reject member strings whose Path::components() contains Component::ParentDir
- [ ] #2 Emit tracing::warn! naming the offending entry
- [ ] #3 Cover with a test using members = ["../escape"]
<!-- AC:END -->
