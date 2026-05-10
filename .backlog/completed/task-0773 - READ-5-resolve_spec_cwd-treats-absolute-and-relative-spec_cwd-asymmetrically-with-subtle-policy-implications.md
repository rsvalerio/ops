---
id: TASK-0773
title: >-
  READ-5: resolve_spec_cwd treats absolute and relative spec_cwd asymmetrically
  with subtle policy implications
status: Done
assignee:
  - TASK-0824
created_date: '2026-05-01 05:56'
updated_date: '2026-05-01 09:54'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/build.rs:158-162`

**What**: For relative ep, joined path = workspace_cwd.join(ep); for absolute, joined = ep.clone() (so canonicalize-under-Deny is skipped at the early-return). Deny canonicalize-to-shrink-TOCTOU path runs only for relative inputs.

**Why it matters**: A hook-path config with cwd = "/abs/path/inside/workspace" benefits less from the SEC-25 mitigation than cwd = "sub/dir", even though both are equally targetable by symlink swap. The asymmetry is undocumented.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Document on CwdEscapePolicy::Deny that the canonicalize-narrowing only applies to relative spec_cwd
- [x] #2 Or extend the canonicalize-on-Deny block to cover absolute inputs as well, preserving the existing test coverage
- [x] #3 Add a regression test for the absolute-inside-workspace path under Deny that pins the chosen behaviour
<!-- AC:END -->
