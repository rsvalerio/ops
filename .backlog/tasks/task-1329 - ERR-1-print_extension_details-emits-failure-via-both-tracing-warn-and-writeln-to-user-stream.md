---
id: TASK-1329
title: >-
  ERR-1: print_extension_details emits failure via both tracing::warn! and
  writeln! to user stream
status: Done
assignee:
  - TASK-1383
created_date: '2026-05-12 16:26'
updated_date: '2026-05-12 23:16'
labels:
  - code-review-rust
  - errors
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/extension_cmd.rs:341-345`

**What**: The data-registry build error branch in `print_extension_details` both calls `tracing::warn!(...)` and writes the same formatted message via `writeln!(w, "\n{msg}")?` to the user-facing stream. ERR-1 requires handling or propagating an error once; here it is reported on two channels simultaneously.

**Why it matters**: Operators running with `RUST_LOG=ops=warn` (or any sink that re-renders tracing) see the same diagnostic twice. The duplication also makes downstream log parsing brittle.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Pick a single emission channel for the data-registry build failure (operator log via tracing OR user-visible writeln!), not both.
- [ ] #2 Add a unit test that asserts only the chosen channel emits when the registry build fails.
<!-- AC:END -->
