---
id: TASK-0207
title: >-
  DUP-1: CLI subcommand dispatch pattern 'load_config_and_cwd +
  build_data_registry' repeats across run_about, run_deps, run_extension_show
status: Done
assignee: []
created_date: '2026-04-22 21:28'
updated_date: '2026-04-23 15:06'
labels:
  - rust-code-review
  - DUP
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Files**:
- `crates/cli/src/subcommands.rs:18-19` (run_about)
- `crates/cli/src/subcommands.rs:44-45` (run_deps)
- `crates/cli/src/extension_cmd.rs:144-145` (run_extension_show_with_tty_check)

**What**: Three handlers begin with the identical 3-line preamble:
```
let (config, cwd) = crate::load_config_and_cwd()?;
let registry = crate::registry::build_data_registry(&config, &cwd)?;
```
(extension_show uses `collect_compiled_extensions` instead of `build_data_registry` but the config+cwd load is identical). TASK-0118 addressed a similar duplication inside cargo-toml; this is the CLI-side analog.

**Why it matters**: DUP-1 (5+ lines identical). Low impact — three sites — but extracting a `with_registry<F>(f: F)` helper (or `CliContext::new()` that owns config+cwd+registry) cleans up the pattern and centralizes error context on the load/build. Small, mechanical refactor.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract a CliContext or with_registry helper; apply to run_about, run_deps, run_extension_show
<!-- AC:END -->
