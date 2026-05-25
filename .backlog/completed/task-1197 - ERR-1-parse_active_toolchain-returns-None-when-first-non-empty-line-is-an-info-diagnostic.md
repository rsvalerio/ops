---
id: TASK-1197
title: >-
  ERR-1: parse_active_toolchain returns None when first non-empty line is an
  info: diagnostic
status: Done
assignee:
  - TASK-1267
created_date: '2026-05-08 08:14'
updated_date: '2026-05-09 14:40'
labels:
  - code-review-rust
  - err
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/probe.rs:109-118`

**What**: parse_active_toolchain finds the first non-empty line, takes its first whitespace token, and rejects the line if that token is one of error: / warning: / info: / note:. On rejection it returns None — but rustup commonly emits a leading info: progress line followed by the real toolchain on the next line.

**Why it matters**: get_active_toolchain feeds install_rustup_component(..., toolchain). A spurious None makes install_tool bail with "could not determine active toolchain" on environments where rustup is perfectly healthy.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 parse_active_toolchain('info: syncing channel updates...\nstable-aarch64-apple-darwin\n...') returns Some(stable-aarch64-apple-darwin), not None.
- [x] #2 parse_active_toolchain('error: no default toolchain configured\n') still returns None (no non-diagnostic line exists).
<!-- AC:END -->
