---
id: TASK-0935
title: >-
  ERR-1: Stack::has_manifest_in_dir uses read_dir().flatten(), silently dropping
  per-entry IO errors during stack detection
status: Done
assignee: []
created_date: '2026-05-02 15:50'
updated_date: '2026-05-02 17:26'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/stack.rs:67-88` (function `has_manifest_in_dir`)

**What**: The extension-based detection path iterates the workspace dir entries via `entries.flatten()` (line ~78), which collapses every `Err(io::Error)` from `dir.read_dir()` into a silent skip. A `.tf` file with a transient `EACCES` (permission-denied parent component, NFS hiccup, ENAMETOOLONG on a junk path) makes Terraform detection silently miss the manifest and fall through to the next stack — no `tracing::debug` breadcrumb, no signal in user-visible diagnostics. This regresses the discipline established for the *outer* `Path::try_exists` probe (TASK-0660 / SEC-25), where the same kind of error is logged at `tracing::debug` so an operator chasing mis-detection has a trail.

**Why it matters**: Direct sibling to TASK-0556 (`resolved_workspace_members swallowed read_dir errors with if-let-Ok and entries.flatten()`, Done) and TASK-0517 (`resolve_member_globs swallows read_dir errors silently`, Done). Both shipped the same calibration: pair the `flatten`/`if let Ok(_)` shape with at least one `tracing::warn!`/`debug!` breadcrumb so the failure is observable. `Stack::detect` is the single function that decides which stack-default command set the user gets — silent fall-through here turns a Terraform manifest with a permission glitch into a generic-stack invocation with zero signal. The outer `Path::try_exists` arm in this same function (the TASK-0660 fix) already does the right thing; the `read_dir` arm regressed the same idiom.

**Acceptance criteria**: replace `entries.flatten()` with an explicit match that logs `tracing::debug!` (consistent with the function's existing `manifest_present` breadcrumb) when a `read_dir` entry surfaces an error, naming the parent directory and the wrapped IO error. Detection still treats the failure as "no match" so detection is monotonic with the prior behavior. Add a regression test (Unix-only) that drops a `.tf` file under a 0o000 chmodded subdir and asserts the warning was emitted. Cross-reference TASK-0556 / TASK-0517 in the fix message so the discipline stays visible.

<!-- scan confidence: high -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Replace entries.flatten() with explicit match that logs tracing::debug per skipped entry
- [x] #2 Log includes parent dir path and the underlying IO error
- [x] #3 Regression test (Unix) drops a .tf file under a 0o000 dir and asserts the breadcrumb is emitted
<!-- AC:END -->
