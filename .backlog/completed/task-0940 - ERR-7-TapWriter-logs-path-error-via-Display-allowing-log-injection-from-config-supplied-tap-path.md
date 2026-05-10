---
id: TASK-0940
title: >-
  ERR-7: TapWriter logs path/error via Display, allowing log injection from
  config-supplied tap path
status: Done
assignee: []
created_date: '2026-05-02 16:02'
updated_date: '2026-05-02 17:26'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/display/tap.rs:31, 92, 101`

**What**: `TapWriter::new` and `append_marker` log `path = %path.display(), error = %e` with the Display formatter at three sites. The tap path comes from `.ops.toml` configuration; a path containing newlines or ANSI escapes can forge or corrupt tracing log lines.

**Why it matters**: Same log-injection class as TASK-0930 (workspace_member_globs) and TASK-0937 (hook-common/git.rs). The TASK-0818 sweep missed this file.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 All three tracing events in tap.rs use Debug formatter (?path.display(), ?e) instead of Display
- [x] #2 Regression test asserts that an embedded \n or \u{1b} in tap path is escaped in tracing output
<!-- AC:END -->
