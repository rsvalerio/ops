---
id: TASK-0297
title: >-
  FN-1: resolve_spec_cwd in crates/runner exec.rs mixes expand, validate,
  escape-check, and policy dispatch
status: Done
assignee:
  - TASK-0298
created_date: '2026-04-23 16:54'
updated_date: '2026-04-23 17:21'
labels:
  - rust-code-review
  - complexity
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/exec.rs:90`

**What**: `resolve_spec_cwd` is ~54 lines and combines four abstraction levels: shell-path expansion, absolute-vs-relative join against workspace, lexical + canonical escape detection against the workspace root, and policy-based warn/error dispatch.

**Why it matters**: FN-1 targets functions >50 lines that mix abstraction levels. Escape detection (SEC-14-adjacent) is security-relevant and deserves to be a named, individually testable helper rather than buried inside path-resolution orchestration.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Escape detection extracted to a named helper (e.g. detect_workspace_escape(joined, workspace) -> EscapeKind)
- [x] #2 Policy dispatch (warn vs error) is a separate helper with its own unit tests
- [x] #3 resolve_spec_cwd reduced to ≤ ~30 lines of orchestration
<!-- AC:END -->
