---
id: TASK-0013
title: "run() in main.rs is 100 lines mixing 4 concerns"
status: Triage
assignee: []
created_date: '2026-04-09 19:25:00'
labels: [rust-code-quality, CQ, FN-1, CL-5, high, crate-cli]
dependencies: []
---

## Description

**Location**: `crates/cli/src/main.rs:78-177`
**Anchor**: `fn run`
**Impact**: The main dispatch function is 100 lines operating at 4 abstraction levels: infrastructure (env args, logging), policy (help interception), business dispatch (match on subcommand with inline registry/tool construction), and stack detection. The `detected_stack` variable introduced at line 86 is used across 75 lines of intervening match arms. Several match arms (`About`, `Deps`) are structurally identical 3-liners calling `load_config_and_cwd()` + `build_data_registry()` inline. The `Tools` arm mixes `#[cfg]`-gating (compile-time) with runtime dispatch.

**Notes**:
Extract phases: (1) a `setup()` or `load_context()` that handles config + stack detection, (2) a `dispatch(ctx, subcommand)` that owns the match tree. The identical `About`/`Deps` arms could share a helper. The `Tools` cfg-gated block could be a separate function to isolate the feature-gate noise from the dispatch logic.
