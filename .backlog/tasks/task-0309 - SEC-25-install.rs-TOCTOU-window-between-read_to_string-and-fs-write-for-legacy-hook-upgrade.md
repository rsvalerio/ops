---
id: TASK-0309
title: >-
  SEC-25: install.rs TOCTOU window between read_to_string and fs::write for
  legacy hook upgrade
status: Done
assignee:
  - TASK-0324
created_date: '2026-04-24 08:53'
updated_date: '2026-04-25 12:41'
labels:
  - rust-code-review
  - security
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: extensions/hook-common/src/install.rs:48-75

**What**: The legacy-hook upgrade reads the existing hook, checks for the legacy marker, then writes. Between read and write the file can be replaced by a non-ops hook.

**Why it matters**: SEC-25 TOCTOU — upgrade path may overwrite a user's custom hook written in the race window.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Either perform read + write on a single OpenOptions handle, or use create_new to a temp file + atomic rename after re-verifying the legacy marker
- [x] #2 Test covers concurrent replacement — upgrade must not clobber non-ops content
<!-- AC:END -->
