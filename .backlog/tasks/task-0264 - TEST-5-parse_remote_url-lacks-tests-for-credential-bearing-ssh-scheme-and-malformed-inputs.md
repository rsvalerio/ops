---
id: TASK-0264
title: >-
  TEST-5: parse_remote_url lacks tests for credential-bearing ssh scheme and
  malformed inputs
status: Done
assignee: []
created_date: '2026-04-23 06:36'
updated_date: '2026-04-23 07:48'
labels:
  - rust-code-review
  - test
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/remote.rs:20`

**What**: Existing tests cover https creds but not ssh://user:tok@, file://, empty-host, IPv6 host forms.

**Why it matters**: Regressions in URL parsing can leak credentials or accept bogus shapes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Add test cases for ssh://u:p@host/o/r, file:///repo, [::1]:22
- [x] #2 Cover malformed scheme rejection
<!-- AC:END -->
