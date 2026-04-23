---
id: TASK-0147
title: >-
  ERR-4: install_hook 'not installed by ops' bail message lacks file-contents
  context
status: Done
assignee: []
created_date: '2026-04-22 21:22'
updated_date: '2026-04-23 07:40'
labels:
  - rust-code-review
  - ERR
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/lib.rs:77-83`

**What**: When an existing hook is not recognized as an ops-managed hook, `install_hook` emits `anyhow::bail!` with only the filename and path — no snippet of the existing script's first line (shebang/first command) that would help the user decide whether to back it up or delete it. The hook file was already read into `existing` at line 64; threading a short excerpt (first N bytes, or the shebang line) into the error would make the remediation actionable without the user having to `cat` the file themselves.

**Why it matters**: Users hit this error during onboarding; the current message forces a manual investigation step that the tool has all the information to avoid.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Error message includes the first line (shebang or first command) of the existing hook
<!-- AC:END -->
