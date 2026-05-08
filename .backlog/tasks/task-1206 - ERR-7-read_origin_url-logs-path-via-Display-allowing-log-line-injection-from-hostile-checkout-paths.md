---
id: TASK-1206
title: >-
  ERR-7: read_origin_url logs path via Display, allowing log-line injection from
  hostile checkout paths
status: Done
assignee:
  - TASK-1259
created_date: '2026-05-08 08:17'
updated_date: '2026-05-08 13:33'
labels:
  - code-review-rust
  - err
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/config.rs:99-115`

**What**: Both tracing::warn! call sites in read_origin_url interpolate `path = %path.display()` (Display) for the IO-error and read-error branches. The path is git_dir.join("config"), where git_dir is derived from find_git_dir walking from cwd; an attacker-controlled checkout path containing newline or ANSI escape would forge log lines or recolour terminal output.

**Why it matters**: The matching helper read_workspace_sidecar and the about provider log paths via the ? formatter (Debug) per the ERR-7 / TASK-0937 sweep specifically to neutralise this surface — read_origin_url is the outlier.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Both tracing::warn! blocks switch from path = %path.display() to path = ?path.display(), matching read_workspace_sidecar and the manifest_io::read_optional_text policy.
- [x] #2 A new unit test pins the Debug-escape behaviour: a path with embedded newline / ANSI rendered via format!('{:?}', p.display()) must contain neither raw '\n' nor '\u{1b}' and must contain '\\n'.
<!-- AC:END -->
