---
id: TASK-0578
title: >-
  READ-5: save_about_fields hardcodes ".ops.toml" in cwd, ignoring workspace
  root
status: Done
assignee:
  - TASK-0640
created_date: '2026-04-29 05:17'
updated_date: '2026-04-29 11:52'
labels:
  - code-review-rust
  - READ
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/about_cmd.rs:74`

**What**: save_about_fields constructs `let config_path = PathBuf::from(".ops.toml")` — the cwd at the moment of the call, not the workspace root the rest of the CLI threads through `crate::cwd()` and `Stack::resolve(...)`. If the user runs `ops about setup` from a subdirectory, the saved .ops.toml lands in the subdirectory rather than next to the loaded config.

**Why it matters**: READ-5/silent inconsistency: rest of CLI anchors on resolved workspace root; this writes a fresh .ops.toml in subdir while loaded config came from `../`. Resulting file is in the wrong place.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 save_about_fields accepts the resolved workspace root and joins .ops.toml to it
- [x] #2 Caller threads same workspace root used for data_registry.about_fields(...)
- [x] #3 Test: running from subdirectory writes to workspace-root .ops.toml
<!-- AC:END -->
