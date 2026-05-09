---
id: TASK-1125
title: >-
  PERF-3: exec_command/exec_command_raw clone spec per spawn before
  build_command_async, defeating Arc-only invariant for spec
status: Done
assignee:
  - TASK-1263
created_date: '2026-05-08 07:28'
updated_date: '2026-05-09 10:53'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: \`crates/runner/src/command/exec.rs:360,421\`

**What**: Both \`exec_command\` and \`exec_command_raw\` call \`spec.clone()\` to move into \`build_command_async(spec.clone(), ...)\`. \`ExecCommandSpec\` carries \`args: Vec<String>\`, \`env: IndexMap<String, String>\`, \`cwd: Option<PathBuf>\`, \`program: String\` — a per-spawn deep clone of every argv string and every env value the user wrote in \`.ops.toml\`.

**Why it matters**: TASK-0462 (OWN-2) wrapped \`cwd\` and \`vars\` in \`Arc\` so the parallel hot path is one refcount bump per field per spawn — but \`spec\` is still deep-cloned, contradicting the explicit "Arc-only inputs, no deep clone" trace in \`build_command_async\` (line 471-476). Under \`MAX_PARALLEL=32\` with composite plans that fan a single command over many leaves (e.g. \`cargo test -p crate1\`, \`-p crate2\`, ...), the per-step allocation profile re-introduces the same \`HashMap\`/\`Vec<String>\` deep-copies TASK-0462 measured. Fixing would mean \`Arc<ExecCommandSpec>\` end-to-end (resolve_exec_specs already returns owned, but could wrap once at parallel.rs:215 boundary).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 exec_command/exec_command_raw no longer take ownership of a freshly cloned ExecCommandSpec on every spawn
- [x] #2 build_command_async signature accepts Arc<ExecCommandSpec> or borrowed reference, matching the Arc-only invariant traced for cwd/vars
- [x] #3 spawn_parallel_tasks does not clone the spec into each tokio task body when an Arc share would suffice
<!-- AC:END -->
