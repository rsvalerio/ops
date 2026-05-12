---
id: TASK-1347
title: >-
  ERR-9: hook_shared::run_hook_install propagates std::env::current_dir() and
  inquire prompt errors without context, surfacing Ctrl-C as an anyhow chain
status: Done
assignee:
  - TASK-1382
created_date: '2026-05-12 16:42'
updated_date: '2026-05-12 22:59'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/hook_shared.rs:45, 98`

**What**:
- Line 45: `let cwd = std::env::current_dir()?;` propagates a bare `io::Error` (e.g. EACCES, ENOENT) with no `with_context` to indicate this is the hook-install path.
- Line 98: `inquire::MultiSelect::new(...).prompt()?` propagates `InquireError::OperationCanceled` (Ctrl-C) as a regular error — the user gets an anyhow message instead of a clean cancel.

**Why it matters**: Hook install is operator-facing; failures should explain which path operation failed and Ctrl-C should exit cleanly without an error chain that looks like a crash. Wrap `current_dir()` with `with_context(|| format!("could not determine cwd while installing {} hook", ops.hook_name))` and pattern-match `InquireError::OperationCanceled` to short-circuit to `Ok(())` (or attach an `ExitCodeOverride(130)` per the convention in `main.rs`).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 current_dir error wrapped with context describing the hook-install step
- [ ] #2 InquireError::OperationCanceled handled distinctly — user-facing cancel is clean (or uses ExitCodeOverride(130))
- [ ] #3 cargo test --workspace passes
<!-- AC:END -->
