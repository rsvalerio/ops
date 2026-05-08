---
id: TASK-1191
title: >-
  ERR-7: init_cmd warn events format paths via Display, missed by the
  project-wide ?path Debug sweep
status: Done
assignee:
  - TASK-1259
created_date: '2026-05-08 08:12'
updated_date: '2026-05-08 13:32'
labels:
  - code-review-rust
  - err
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/init_cmd.rs:33`

**What**: `tracing::warn!("{} already exists; not overwriting (use --force to overwrite)", path.display())` formats the path through Display. The TASK-0944 / TASK-0945 sweep deliberately switched manifest-probe paths to ?path.display() Debug formatting so newlines / ANSI in CWD-derived paths cannot forge log records — but init_cmd was missed.

**Why it matters**: path is `cwd.join(".ops.toml")` where cwd is user-controlled; a directory containing newlines / ANSI escapes lets a malicious working directory smuggle a forged log entry through the operator's structured-log pipeline.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The warn event in run_init_to (and the parent-fsync warn events in write_init) use ?path.display() / ?parent.display() Debug formatting.
- [x] #2 Regression test mirrors stack_detection_path_debug_escapes_control_characters for the init-cmd warn path.
<!-- AC:END -->
