---
id: TASK-0172
title: >-
  READ-8: CLI uses direct writeln!(io::stderr(), ...) for user-facing
  diagnostics instead of a unified reporter
status: Done
assignee: []
created_date: '2026-04-22 21:24'
updated_date: '2026-04-23 15:14'
labels:
  - rust-code-review
  - READ
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Files**:
- `crates/cli/src/main.rs:65, 116` (Error printing, cwd error)
- `crates/cli/src/subcommands.rs:66, 72, 109, 117, 146` (hook skip messages)
- `crates/cli/src/hook_shared.rs:30, 43` (config load warning, no-commands-found)
- `crates/cli/src/about_cmd.rs:55` (summary)

**What**: The CLI mixes three output channels: `tracing::warn!/info!/debug!` (structured logs, filtered by `OPS_LOG_LEVEL`), direct `writeln!(io::stderr(), ...)` (always-on user messages), and `writeln!(io::stdout(), ...)` (command output). The direct-stderr writes are neither logs nor command output — they are user-facing diagnostics that the user expects unconditionally. But nothing in the code makes that policy explicit, and the `let _ = writeln!(...)` pattern silently discards broken-pipe/IO errors.

**Why it matters**: READ-8 guidance prefers `tracing` for service/application diagnostics, but for interactive CLI user messages a dedicated thin reporter layer (`ops_core::ui::note!`, `ops_core::ui::error!`) is idiomatic and enforces consistent formatting (e.g. "ops: warning: ..." prefix) and consistent stderr vs stdout channel selection. Today the same message format varies across files ("ops: warning: ...", "[run-before-commit] ...", bare "No commands found.").
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Introduce a thin ops_core::ui reporter and migrate writeln!(io::stderr(), ...) diagnostics
<!-- AC:END -->
