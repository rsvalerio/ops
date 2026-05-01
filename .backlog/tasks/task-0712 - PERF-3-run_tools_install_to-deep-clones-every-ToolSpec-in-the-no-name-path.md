---
id: TASK-0712
title: 'PERF-3: run_tools_install_to deep-clones every ToolSpec in the no-name path'
status: Done
assignee:
  - TASK-0741
created_date: '2026-04-30 05:30'
updated_date: '2026-04-30 19:36'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/tools_cmd.rs:130-136`

**What**: When `name` is `None`, `run_tools_install_to` builds `tools_to_install: IndexMap<String, ToolSpec>` by walking `config.tools.iter().map(|(k, v)| (k.clone(), v.clone())).collect()`. Every key (`String`) and every `ToolSpec` (which is an enum carrying owned strings: description, optional package name, optional rustup_component) is cloned, even though `tools_to_install` is then immediately passed by reference to `collect_tools(&tools_to_install)` and `install_missing_tools(&missing, &tools_to_install, ...)`. The function never mutates `tools_to_install`; cloning the entire config sub-map is unnecessary copy work on the install hot path and a conceptual smell — clone-as-borrow-checker-workaround (OWN-8) where a borrow of `&config.tools` would suffice for the no-name branch.

**Why it matters**: For users with many tools (the default Rust stack ships ~10), every `ops tools install` allocates a fresh string for each tool name plus a deep clone of each ToolSpec for no benefit. More importantly, the symmetry is misleading: the named path constructs a 1-entry map (legitimate ownership transfer), the unnamed path clones the whole map (no ownership reason). Either restructure both branches around `&IndexMap<String, ToolSpec>` or document why ownership is required.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Refactor run_tools_install_to so the no-name branch does not deep-clone every ToolSpec; pass &config.tools through to collect_tools and install_missing_tools
- [x] #2 Named path keeps its current owning IndexMap shape only if a code comment explains why ownership is needed there but not in the unnamed path
<!-- AC:END -->
