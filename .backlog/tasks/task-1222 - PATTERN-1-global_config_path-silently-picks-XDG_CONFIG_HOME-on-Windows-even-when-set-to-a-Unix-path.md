---
id: TASK-1222
title: >-
  PATTERN-1: global_config_path silently picks XDG_CONFIG_HOME on Windows even
  when set to a Unix path
status: Done
assignee:
  - TASK-1270
created_date: '2026-05-08 12:57'
updated_date: '2026-05-10 17:01'
labels:
  - code-review-rust
  - patterns
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/loader.rs:283-299`

**What**: The XDG_CONFIG_HOME branch is taken on every platform before the Windows %APPDATA% fallback. A Windows user inheriting a Unix-style XDG_CONFIG_HOME (WSL leakage, dotfile sync) loses the documented %APPDATA%\\ops\\config.toml location with no warning.

**Why it matters**: The doc claims XDG honoured cross-platform as a feature, but on Windows there is no convention that XDG points to a Windows-valid path; a stale Unix path silently disables global config. At minimum a debug breadcrumb should record the source of the chosen base.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Log the source of config_dir (XDG vs APPDATA vs HOME) at debug level
- [ ] #2 Validate the path is at least non-empty and absolute
- [ ] #3 Document the WSL-leakage edge case in the function rustdoc
<!-- AC:END -->
