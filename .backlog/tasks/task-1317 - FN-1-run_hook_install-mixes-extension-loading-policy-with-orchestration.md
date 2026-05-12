---
id: TASK-1317
title: 'FN-1: run_hook_install mixes extension-loading policy with orchestration'
status: Done
assignee:
  - TASK-1382
created_date: '2026-05-11 20:31'
updated_date: '2026-05-12 22:59'
labels:
  - code-review-rust
  - structure
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/hook_shared.rs:42-107`

**What**: `pub fn run_hook_install(config: &Config, ops: &HookOps) -> anyhow::Result<()>` is 66 lines that bundle three different responsibilities:

1. Preconditions: TTY check, cwd, git-dir resolution, stack detection (lines 43-49).
2. Extension loading with a non-trivial degradation policy (lines 51-87) — 37 lines that decide between a hard error (when `config.extensions.enabled` is set explicitly) and a soft warn-and-continue path.
3. UI prompt + hook write-out (lines 89-106).

The middle block is the largest and most error-prone part; it owns its own policy, error formatting, and warning message but is inlined here instead of being a named function.

**Why it matters**: FN-1 sets a 50-line soft cap so each function captures one decision. The current shape forces a reader to load the degradation policy whenever they trace hook install. Extracting the extension-loading step would also let the unit tests cover the policy decision directly without going through the interactive prompt.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract a helper (e.g. fn load_hook_extensions(config, cwd, hook_name) -> anyhow::Result<CommandRegistry>) that encapsulates the load + degradation policy at lines 51-87
- [ ] #2 run_hook_install body reduces to roughly 35-40 lines of orchestration
- [ ] #3 Existing behavior preserved: hard error when config.extensions.enabled is Some, soft UI warn otherwise; exactly one operator-facing warning per failure (no double-emit through tracing + ui)
- [ ] #4 cargo clippy --all-targets -- -D warnings and cargo test --workspace pass
<!-- AC:END -->
