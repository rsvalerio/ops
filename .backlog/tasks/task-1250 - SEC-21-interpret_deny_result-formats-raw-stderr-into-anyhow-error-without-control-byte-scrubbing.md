---
id: TASK-1250
title: >-
  SEC-21: interpret_deny_result formats raw stderr into anyhow error without
  control-byte scrubbing
status: Done
assignee:
  - TASK-1259
created_date: '2026-05-08 13:01'
updated_date: '2026-05-08 13:27'
labels:
  - code-review-rust
  - security
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/parse.rs:521`

**What**: The exit-2 arm bails with `stderr.trim()` formatted via Display directly into the anyhow error. cargo-deny stderr text mode and operator deny.toml diagnostics can carry ANSI escapes / newlines / U+0000 sourced from registry-fetched advisory text, then surface verbatim into operator logs and CLI output. Sibling arms route through `truncate_for_log` but it also uses Display.

**Why it matters**: Same threat class as SEC-21 / TASK-1184 (`print_exec_spec`) — attacker-controlled supply-chain bytes can repaint the terminal or forge log entries through `ops deps` failure paths. Other arms of `interpret_deny_result` and `interpret_upgrade_output` share the issue.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Scrub control bytes (or use ? Debug formatter) on every stderr surface in interpret_deny_result and interpret_upgrade_output
- [x] #2 Unit test pinning \n, \x1b[31m, U+0000 in stderr cannot reach the rendered error
- [x] #3 Align with the workspace-glob breadcrumb policy from query.rs
<!-- AC:END -->
