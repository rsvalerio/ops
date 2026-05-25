---
id: TASK-1525
title: >-
  PERF-3: deps SeverityClass::style allocates owned String even when colouring
  is a no-op
status: Done
assignee:
  - TASK-1646
created_date: '2026-05-19 07:32'
updated_date: '2026-05-25 17:58'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/format.rs:61-68`

**What**: `style()` always calls `.into_owned()` on the `Cow` returned by `red`/`yellow`/`dim`, materialising a fresh String per row regardless of TTY/colour state. Combined with one call per row in `format_severity_section` and `format_bans_summary`, this is a per-row allocation that defeats the PERF-3/TASK-0802 "one allocation per render" intent the file's header comment claims.

**Why it matters**: The module-level comment explicitly advertises allocation discipline; this helper silently breaks it on the hottest formatting path.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Return Cow<'_, str> from style (and from colorize_severity)
- [ ] #2 writeln! interpolates the Cow directly without forcing ownership
- [ ] #3 No String allocations per row when colour is disabled (verify in --release or via test)
<!-- AC:END -->
