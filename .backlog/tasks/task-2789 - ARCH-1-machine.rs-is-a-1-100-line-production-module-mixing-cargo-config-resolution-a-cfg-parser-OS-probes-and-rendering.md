---
id: TASK-2789
title: 'ARCH-1: machine.rs is a 1,100-line production module mixing cargo-config resolution, a cfg parser, OS probes and rendering'
status: To Do
assignee: []
created_date: '2026-11-18 18:45'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - ARCH
dependencies: []
parent_task_id: 'TASK-2857'
modified_files:
  - extensions/about/src/machine.rs
priority: medium
ordinal: 1000
dedup_key: 'ARCH-1:extensions/about/src/machine.rs:module'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/machine.rs:1-1121` (production code before `#[cfg(test)]` at line 1122; 1,772 lines with tests)

**What**: One module holds four unrelated concerns: (1) cargo config layer discovery, include expansion and setting resolution (`config_layers`, `push_with_includes`, `resolve_cargo_settings`, `resolve_target_tables`, `merge_list`, ~lines 135-742); (2) a recursive-descent `cfg(..)` evaluator (`CfgParser`, lines 433-530); (3) OS probes and their parsers (`ps`, `/proc/loadavg`, `sysctl`, mounts, `df`, `sccache`, lines 744-959); (4) the report types, text rendering and the command entry point. It exports well over 10 public items.

**Why it matters**: ARCH-1 red flags are >500 lines and mixed concerns; this is roughly double the line threshold. Each concern changes for different reasons (cargo semantics vs. platform probing vs. output), and the size makes each harder to review — several findings in this module (Display log fields, uncapped reads) are exactly the drift a smaller, focused module makes visible.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 machine is split into submodules by concern (e.g. machine/cargo_config.rs, machine/cfg_expr.rs, machine/probe.rs, machine/render.rs) with the public API unchanged
- [ ] #2 No resulting production module exceeds ~500 lines
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.22 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
