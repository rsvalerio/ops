---
id: TASK-012
title: "run_tools_install_to exceeds 50-line threshold at 90 lines"
status: To Do
assignee: []
created_date: '2026-04-07 12:00:00'
labels: [rust-code-quality, CQ, FN-1, medium, effort-S, crate-cli]
dependencies: []
---

## Description

**Location**: `crates/cli/src/tools_cmd.rs:112-201`
**Anchor**: `fn run_tools_install_to`
**Impact**: At 90 lines this function is nearly double the 50-line threshold, mixing tool resolution, installation loop with counters, and summary formatting into a single function body.

**Notes**:
The function has three distinct phases that map naturally to extraction:
1. **Resolution** (lines 119-144): resolve tools to install, filter missing, early-return if none.
2. **Installation loop** (lines 148-180): iterate missing tools, attempt install, track counters.
3. **Summary** (lines 182-200): format result message and choose exit code.

Extract phases 2 and 3 into helpers (e.g., `install_missing_tools`, `format_install_summary`) to bring each function under 50 lines and reduce cognitive load.
