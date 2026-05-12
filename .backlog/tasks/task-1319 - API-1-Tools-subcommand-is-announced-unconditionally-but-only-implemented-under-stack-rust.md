---
id: TASK-1319
title: >-
  API-1: Tools subcommand is announced unconditionally but only implemented
  under stack-rust
status: To Do
assignee:
  - TASK-1385
created_date: '2026-05-11 20:35'
updated_date: '2026-05-12 22:16'
labels:
  - code-review-rust
  - API
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/args.rs:124-128`, `crates/cli/src/main.rs:241-251`

**What**: `CoreSubcommand::Tools` is declared without a `#[cfg]` gate in `args.rs` (lines 124-128), unlike its siblings `Deps` (line 93, `#[cfg(feature = "stack-rust")]`) and `Plans` (line 135, `#[cfg(feature = "stack-terraform")]`). The handler in `main.rs:241-251` is split — it dispatches to `run_tools(...)` under `#[cfg(feature = "stack-rust")]` and bails with `anyhow::bail!("tools subcommand requires the stack-rust feature")` otherwise.

The consequences in a default-features build (no `stack-rust`):
- `ops tools list` parses successfully (clap accepts it) and runs to completion before failing at runtime.
- `ops --help` lists `tools` as a top-level subcommand, but invoking it surfaces a runtime error rather than clap's standard "unknown subcommand" rejection that `deps` gets.
- The CLI surface advertises a capability the binary cannot honour. `args::stack_specific_commands` (line 211) only hides `tools` *when* `stack-rust` is compiled in — so under a no-`stack-rust` build the hiding pass is a no-op for `tools`.

`Deps` already demonstrates the correct shape: gate the variant *and* the dispatch arm on the same `#[cfg]`, so clap reports "unknown subcommand" instead of the handler running and bailing.

**Why it matters**: API surface should fail at the earliest possible layer — parsing for a non-existent feature, not runtime. The current shape (1) makes `ops --help` lie about what the binary can do and (2) routes the failure through `anyhow::bail` rather than clap, so the error message and exit-code path differ from the symmetric `deps` case. Fixing it brings `Tools` in line with `Deps` and `Plans` and shrinks the help output for builds that genuinely lack the feature.

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 CoreSubcommand::Tools variant is gated on #[cfg(feature = "stack-rust")] (matching the Deps/Plans pattern in args.rs)
- [ ] #2 The dispatch arm in main.rs no longer needs the not(feature = "stack-rust") bail branch; the entire arm is feature-gated
- [ ] #3 Under a no-stack-rust build, invoking the tools subcommand fails with clap's standard unknown-subcommand error (exit 2), not an anyhow runtime bail
- [ ] #4 Under a no-stack-rust build, top-level help output does not list "tools" as an available subcommand
<!-- AC:END -->
