---
id: TASK-0427
title: >-
  ERR-1: cli/main.rs loads .ops.toml twice per invocation with divergent error
  policies
status: To Do
assignee:
  - TASK-0536
created_date: '2026-04-28 04:41'
updated_date: '2026-04-28 07:14'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/main.rs:110` (early load) and `crates/cli/src/main.rs:200-204` (load_config_and_cwd re-loads)

**What**: `run()` loads `early_config = ops_core::config::load_config_or_default("early")` for help/stack detection, then dispatches to handlers (`run_external_command` -> build_runner -> load_config_and_cwd, run_about -> cli_data_context -> load_config_and_cwd, run_hook_dispatch -> its own load_config()) which all call `load_config()` a second time. The two reads can disagree if .ops.toml changes mid-invocation, and the second call uses load_config() (hard error) where the first used load_config_or_default (warn-and-default), so the same user config can succeed on the early path and bail later — confusing diagnostic.

**Why it matters**: Inconsistent config state between help rendering and command dispatch; redundant TOML parse on every CLI call; the divergent error policies (_or_default vs hard Result) defeats the unification work TASK-0345 / TASK-0267 / TASK-0240 already did for individual handlers.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Thread early_config (or a pre-resolved Config) through dispatch into the handlers so they do not re-load .ops.toml
- [ ] #2 All handlers consult the same loaded Config for the lifetime of one CLI invocation
- [ ] #3 Add a regression test that asserts load_config is not invoked twice during a typical ops <cmd> flow
<!-- AC:END -->
