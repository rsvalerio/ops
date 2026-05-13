---
id: TASK-1434
title: >-
  READ-1: home_dir Windows precedence prefers HOME over USERPROFILE without a
  documented rationale
status: To Do
assignee:
  - TASK-1455
created_date: '2026-05-13 18:23'
updated_date: '2026-05-13 19:09'
labels:
  - code-review-rust
  - READ
  - ARCH
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/paths.rs:31`

**What**: On non-Unix, `HOME` (Git Bash / WSL / MSYS leak) wins over `USERPROFILE`. The existing doc-comment names the WSL/MSYS source but does not state the security or compat trade-off (a process inheriting a Unix-style HOME via WSL pollution gets that path even on native Windows).

**Why it matters**: Sister concern to the XDG_CONFIG_HOME WSL-leakage edge case the config loader documents. Either the project should accept the same trade-off everywhere with one shared rationale, or align with the cross-platform-tools convention (USERPROFILE first on native Windows).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Decide and document the Windows precedence policy (HOME-first vs USERPROFILE-first) with explicit rationale
- [ ] #2 global_config_path and home_dir use the same precedence
<!-- AC:END -->
