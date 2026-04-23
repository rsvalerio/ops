---
id: TASK-0282
title: >-
  API-1: run_external_command silently drops non-UTF8 command names via
  filter_map to_str
status: To Do
assignee: []
created_date: '2026-04-23 06:37'
updated_date: '2026-04-23 06:46'
labels:
  - rust-code-review
  - api
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/run_cmd.rs:31`

**What**: Non-UTF8 OsString entries vanish then bail says "missing command name" without context.

**Why it matters**: User sees generic error when real cause is encoding.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Error explicitly on non-UTF8
- [ ] #2 Or document UTF-8 requirement in help
<!-- AC:END -->
