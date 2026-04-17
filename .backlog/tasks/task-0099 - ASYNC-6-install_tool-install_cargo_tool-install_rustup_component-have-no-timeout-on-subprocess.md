---
id: TASK-0099
title: >-
  ASYNC-6: install_tool / install_cargo_tool / install_rustup_component have no
  timeout on subprocess
status: To Do
assignee: []
created_date: '2026-04-17 11:57'
updated_date: '2026-04-17 12:07'
labels:
  - rust-code-review
  - async
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/lib.rs` (fns `install_cargo_tool` ~line 142, `install_rustup_component` ~line 164, `install_tool` ~line 192)

**What**: Each of these helpers calls `Command::new(...).args(...).status()` without any timeout or cancellation. A hung `cargo install` (registry fetch stalled, crates.io rate-limit, proxy hang, flaky mirror) blocks the whole `ops tools install` flow indefinitely — there is no upper bound, no progress check, and no way for the user to recover except `Ctrl+C`. This is an external-call robustness gap.

**Why it matters**: Violates ASYNC-6 ("timeouts + retries + exponential backoff for all external calls") in spirit — the rule explicitly applies to any external call, even synchronous ones. A CI job invoking `ops tools install` can exhaust its time budget silently. A user script gets wedged on a dead network. The failure is also opaque: the user cannot distinguish "compiling (slow)" from "hung".

**Notes**: Retries and backoff are handled by cargo itself for registry fetches — the remaining concern is a wall-clock ceiling so a hung install surfaces as a clear error, not a freeze.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 install_cargo_tool and install_rustup_component enforce a configurable wall-clock timeout (default: generous, e.g. 10 min) around the subprocess, returning a clear error on timeout
- [ ] #2 Timeout is surfaced as a distinct error message ("cargo install <name> timed out after N seconds"), not a generic failure
- [ ] #3 Tests cover the timeout path using a mocked long-running subprocess (e.g.  spawned by a shell) without requiring real cargo
- [ ] #4 AGENTS-style run of All tools already installed on a throttled mirror surfaces the timeout within the configured limit instead of hanging
<!-- AC:END -->
