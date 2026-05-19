---
id: TASK-1523
title: >-
  ERR-4: deps DepsProvider::provide drops error source chain via anyhow!("...:
  {}", e)
status: To Do
assignee:
  - TASK-1574
created_date: '2026-05-19 07:32'
updated_date: '2026-05-19 16:45'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/lib.rs:319-328`

**What**: Both `run_cargo_upgrade_dry_run` and `run_cargo_deny` errors are stringified into a fresh `anyhow::anyhow!`, severing the source chain so `Error::source()` walking and `{:?}` Debug printing lose the underlying cause.

**Why it matters**: Operators chasing a deps failure via `RUST_BACKTRACE=1` or `anyhow`'s chain printer get only the top message — the real cargo subprocess error (timeout, I/O, exit-code) becomes a flat string.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace with .with_context(|| "cargo upgrade failed") / .with_context(|| "cargo deny failed") then .map_err(DataProviderError::from)
- [ ] #2 Source chain preserved end-to-end (verify with a unit test asserting err.source().is_some())
- [ ] #3 No change to the user-facing top-level message
<!-- AC:END -->
