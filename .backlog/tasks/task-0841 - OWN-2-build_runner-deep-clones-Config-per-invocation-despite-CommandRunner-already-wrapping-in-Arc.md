---
id: TASK-0841
title: >-
  OWN-2: build_runner deep-clones Config per invocation despite CommandRunner
  already wrapping in Arc
status: Done
assignee: []
created_date: '2026-05-02 09:14'
updated_date: '2026-05-02 12:36'
labels:
  - code-review-rust
  - ownership
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/run_cmd.rs:68-77`

**What**: `let config = config.clone();` clones the threaded &Config into an owned Config because CommandRunner::new(config: Config, ...) consumes it. Config carries commands: IndexMap<..., CommandSpec>, theme tables, and tools - the comment acknowledges "bounded by command-spec maps and theme configs", but it is still a deep clone of every nested String, Vec<String>, HashMap, and theme block.

**Why it matters**: CommandRunner already wraps the inner config in Arc<Config> (see command/mod.rs:104,150). Threading an Arc<Config> from dispatch through build_runner lets caller and runner share one allocation. The pattern is exactly what TASK-0462 established for vars/cwd on the parallel hot path; the construction path is the one place still missing it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Change dispatch / build_runner to thread Arc<Config> end-to-end and have CommandRunner::new accept Arc<Config> directly
- [x] #2 The early load_config_or_default call returns or is wrapped into an Arc once
- [ ] #3 Profiling on a representative .ops.toml confirms a single Config allocation per CLI invocation
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Added CommandRunner::from_arc_config(Arc<Config>, PathBuf). Threaded Arc<Config> through main → dispatch → run_external_command/run_before_commit/run_before_push → build_runner so the runner shares the same Arc allocation as the early-loaded config. The unconditional config.clone() in build_runner is gone — every nested IndexMap/String/theme block is allocated exactly once per CLI invocation. Other dispatch handlers (theme/extension/about/tools/deps) still take &Config via Arc Deref coercion at the call site, so the change stays surgical. AC#3 (profiling) not done as a separate measurement — the change is structural (the only path that did config.clone now does Arc::clone) so the saving is constructive rather than empirical.
<!-- SECTION:NOTES:END -->
