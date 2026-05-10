---
id: TASK-0930
title: >-
  ERR-7: workspace_member_globs warn logs path via Display, regressing TASK-0818
  sweep
status: Done
assignee: []
created_date: '2026-05-02 15:33'
updated_date: '2026-05-02 17:12'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/units.rs:92-96`

**What**: When `package.json` parsing fails inside `workspace_member_globs`, the warn event uses `path = %pkg_path.display()` (Display) instead of `path = ?pkg_path.display()` (Debug). The sister site in `package_json.rs:84-89` uses Debug per TASK-0818, and the parallel go-side fix landed in TASK-0809. This site was missed when TASK-0818 swept the manifest-parse warnings.

**Why it matters**: A crafted project root path containing `\n` or ANSI escapes can forge multi-line/colored log records when an operator runs About in a directory whose path is attacker-influenced (CI checkouts, container mounts). Same log-injection class TASK-0818 was filed to close.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 workspace_member_globs warn event in extensions-node/about/src/units.rs uses the Debug formatter for the path field (matches package_json.rs pattern).
- [x] #2 A regression test asserts the formatted log value escapes embedded \n and \u{1b} (mirrors package_json_path_debug_escapes_control_characters).
<!-- AC:END -->
