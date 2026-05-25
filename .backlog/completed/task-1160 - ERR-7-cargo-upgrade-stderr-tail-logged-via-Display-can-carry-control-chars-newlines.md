---
id: TASK-1160
title: >-
  ERR-7: cargo-upgrade stderr tail logged via Display can carry control chars /
  newlines
status: Done
assignee:
  - TASK-1259
created_date: '2026-05-08 07:44'
updated_date: '2026-05-08 13:27'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/parse.rs:81`

**What**: `interpret_upgrade_output` non-zero branch builds `anyhow::bail!(\"...stderr (truncated): {}\", truncate_for_log(stderr.trim()))` using Display formatter, where `truncate_for_log` is plain truncation with no escaping. Sister code paths in this codebase route attacker-controllable strings through `?` (Debug) precisely to escape newlines/ANSI (TASK-0941, TASK-0977). cargo-upgrade stderr can include registry-served content with embedded ANSI/newlines.

**Why it matters**: Same log-injection class as elsewhere in the workspace (e.g. `query.rs:368` uses `?`-format on similar data). Internal inconsistency that re-introduces the SEC-21-class log-injection surface the rest of the codebase has systematically closed.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Switch to Debug-format ({:?}) for the stderr tail so newlines / ANSI escape
- [x] #2 Mirror the change at parse.rs:537 (unrecognised exit code arm) and parse.rs:514 (zero diagnostics arm)
- [x] #3 Pin with a unit test analogous to stderr_snippet_debug_escapes_control_characters (probe.rs:600)
<!-- AC:END -->
