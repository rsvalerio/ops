---
id: TASK-1354
title: >-
  READ-5: run_hook_install hardcodes io::stdout() while a parallel
  run_hook_install_with(&mut dyn Write) already exists for tests
status: Done
assignee:
  - TASK-1382
created_date: '2026-05-12 21:28'
updated_date: '2026-05-12 22:59'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/hook_shared.rs:102`

**What**: The production `run_hook_install` does `let mut w = io::stdout(); (ops.install_hook)(&git_dir, &mut w)?; (ops.ensure_config_command)(&cwd, &selected, &mut w)?;`, but the immediately-adjacent `#[cfg(test)] run_hook_install_with` (line 111) already takes `w: &mut dyn Write`. Two install paths exist solely so tests can capture output.

**Why it matters**: The duplication forces test-only orchestration to diverge from production (different signatures, drift risk), and the production happy-path messages are unobservable except via integration tests that re-spawn the binary. Folding the test variant up into the production entry collapses both paths to one and matches the `_to` pattern used elsewhere in this crate.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Change run_hook_install to accept w: &mut dyn Write (callers pass &mut std::io::stdout()) and delete run_hook_install_with
- [ ] #2 Verify pre_hook_cmd callers compile and existing tests now exercise the production entry point with a buffer writer
<!-- AC:END -->
