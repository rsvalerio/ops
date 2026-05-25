---
id: TASK-1352
title: 'ERR-9: classify_confirm_result converts inquire errors without .context()'
status: Done
assignee:
  - TASK-1382
created_date: '2026-05-12 16:48'
updated_date: '2026-05-12 22:59'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/subcommands.rs:91-101`

**What**: `classify_confirm_result` distinguishes `OperationCanceled`/`OperationInterrupted` (returned as `Ok(None)`) from real failures, but for the latter it does a bare `Err(e.into())`:

```rust
fn classify_confirm_result(
    res: Result<bool, inquire::InquireError>,
) -> anyhow::Result<Option<bool>> {
    match res {
        Ok(b) => Ok(Some(b)),
        Err(
            inquire::InquireError::OperationCanceled | inquire::InquireError::OperationInterrupted,
        ) => Ok(None),
        Err(e) => Err(e.into()),
    }
}
```

The single caller is `prompt_hook_install` (subcommands.rs:131), which gates the "Run `ops <hook> install` now?" prompt. A non-cancel `inquire` error here (NotTTY, IO, parse) reaches `main` as a bare `inquire: <variant text>`, with no breadcrumb saying *which prompt* failed.

**Why it matters**: ERR-9 (context on `?`-propagated errors). Mirrors the rationale in TASK-1347 (`hook_shared::run_hook_install` propagating inquire/current_dir errors without context) but applies to the install-prompt site that TASK-1325 also touches. Attaching `.context(format!("install prompt for {hook_name} failed"))` (or similar) makes the failure self-locating.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Err(e) branch in classify_confirm_result attaches anyhow context naming the prompt source so a NotTTY / IO error tells the user which prompt was in flight
- [ ] #2 the existing classify_confirm_result_real_error_propagates test continues to pass and asserts the new context string
<!-- AC:END -->
