---
id: TASK-1650
title: >-
  ERR-4: import-makefile read_to_string flattens io::Error via anyhow! instead
  of .with_context
status: Done
assignee: []
created_date: '2026-06-07 10:53'
updated_date: '2026-06-07 11:32'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/import_makefile_cmd.rs:48`

**What**: `std::fs::read_to_string(&makefile_path).map_err(|e| anyhow::anyhow!("could not read {}: {e}", makefile_path.display()))` formats the `io::Error` into the message string, discarding it as a typed `source()` in the error chain.

**Why it matters**: Wave-140 (SEC-21/TASK-1531) just established the convention of preserving the error source chain via `.context()`/`.with_context()` instead of `anyhow!("... {e}")` interpolation. This new file regresses that: callers inspecting the chain (e.g. `downcast_ref::<io::Error>()` for `ErrorKind::NotFound`) cannot, and `{:#}`/debug rendering loses the structured cause.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 read_to_string failure is propagated with .with_context(|| format!("could not read {}", makefile_path.display())) preserving the io::Error as source
- [x] #2 Rendered CLI error message remains equivalent (path + OS error text)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
read_to_string failure now uses .with_context(|| format!("could not read {}", path.display())), preserving io::Error as source(); CLI renders {e:#} so the message (path + OS error) is unchanged.
<!-- SECTION:NOTES:END -->
