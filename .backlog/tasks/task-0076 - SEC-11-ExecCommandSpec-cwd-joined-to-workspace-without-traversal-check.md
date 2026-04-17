---
id: TASK-0076
title: 'SEC-11: ExecCommandSpec cwd joined to workspace without traversal check'
status: To Do
assignee: []
created_date: '2026-04-17 11:32'
updated_date: '2026-04-17 12:07'
labels:
  - rust-codereview
  - sec
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/exec.rs:210`

**What**: build_command expands spec.cwd, and if relative, joins it to runner cwd without normalizing or rejecting parent-path components like `..`.

**Why it matters**: User-supplied cwd values can escape the workspace via `../`; asymmetric with other sensitive-env heuristics.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Canonicalize the resolved cwd and log a warning (or bail) when it escapes workspace root
- [ ] #2 Document the decision alongside SEC-001/002/003 security notes
<!-- AC:END -->
