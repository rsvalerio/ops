---
id: TASK-0017
title: "run_tools_install_to is 90 lines with mixed concerns and 3 overlapping data structures"
status: Triage
assignee: []
created_date: '2026-04-09 19:25:00'
labels: [rust-code-quality, CQ, FN-1, CL-5, medium, crate-cli]
dependencies: []
---

## Description

**Location**: `crates/cli/src/tools_cmd.rs:112-201`
**Anchor**: `fn run_tools_install_to`
**Impact**: The function combines config loading, tool spec resolution, status filtering, UI output, installation side-effects, result tallying, and summary formatting. The reader must simultaneously track the config's `tools` map (for spec lookup), the filtered `tools` vec (for status), and the `tools_to_install` IndexMap (for install specs) — three collections representing overlapping views of the same data. The `let Some(spec) = tools_to_install.get(...)` lookup inside the install loop is necessary because `missing` is a `Vec<&ToolInfo>` with no spec attached.

**Notes**:
Extract the install loop into `fn install_tools(tools: &[...], specs: &IndexMap<...>, w: &mut dyn Write) -> (usize, usize)` returning `(installed, failed)`. This leaves the outer function as a coordinator. Consider attaching the install spec to the `ToolInfo` or passing a combined iterator to avoid the disconnected data structures.
