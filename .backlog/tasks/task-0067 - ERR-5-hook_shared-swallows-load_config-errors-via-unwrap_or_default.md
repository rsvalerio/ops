---
id: TASK-0067
title: 'ERR-5: hook_shared swallows load_config errors via unwrap_or_default'
status: To Do
assignee: []
created_date: '2026-04-17 11:30'
updated_date: '2026-04-17 12:07'
labels:
  - rust-codereview
  - err
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/hook_shared.rs:24`

**What**: `run_hook_install` calls `load_config().unwrap_or_default()`; any parse error in .ops.toml is silently discarded before offering commands to select.

**Why it matters**: Users configuring hooks with a broken local config see an empty/wrong command list without any warning.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace unwrap_or_default with match/if-let that logs a warning when load fails
- [ ] #2 Surface the parse error to stderr so users can fix the config
<!-- AC:END -->
