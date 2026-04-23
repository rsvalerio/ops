---
id: TASK-0160
title: >-
  ERR-1: run_with_runtime constructs a fresh tokio Runtime per subcommand
  invocation
status: Done
assignee: []
created_date: '2026-04-22 21:23'
updated_date: '2026-04-23 14:59'
labels:
  - rust-code-review
  - ERR
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/run_cmd.rs:52-58` (run_with_runtime)

**What**: `run_with_runtime` builds a new `tokio::runtime::Runtime` every time `run_command`, `run_command_cli`, `run_command_raw`, and `run_commands` are called. In `run_commands` this happens exactly once, but the helper pattern invites callers to think of runtime construction as cheap. More importantly, `Runtime::new()` may fail (EMFILE, ENOMEM, epoll init error) and the `?` surfaces a bare `std::io::Error` without context like "failed to start tokio runtime for command execution".

**Why it matters**: Low-severity ERR-4 / ERR-1: users hitting resource limits get an opaque "Too many open files (os error 24)" instead of actionable context. Fix: wrap with `.context("failed to start tokio runtime")` in `run_with_runtime`. Additionally consider `#[tokio::main]` on a dedicated async entry point so the runtime is constructed exactly once per process — this also lets subcommands share a runtime if needed.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 run_with_runtime wraps Runtime::new() with .context()
<!-- AC:END -->
