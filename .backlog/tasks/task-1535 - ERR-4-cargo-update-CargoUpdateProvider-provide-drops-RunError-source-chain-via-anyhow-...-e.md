---
id: TASK-1535
title: >-
  ERR-4: cargo-update CargoUpdateProvider::provide drops RunError source chain
  via anyhow!("...: {}", e)
status: To Do
assignee:
  - TASK-1575
created_date: '2026-05-19 09:54'
updated_date: '2026-05-19 16:46'
labels:
  - code-review-rust
  - err
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-update/src/lib.rs:425-427`

**What**: `provide()` converts a `RunError` from `run_cargo_update_dry_run` with `anyhow::anyhow!("cargo update --dry-run failed: {}", e)`. The `{}` formatter flattens the source to its Display form, so the resulting `DataProviderError` carries no `source()` chain — the underlying `io::Error` (or `Timeout` variant detail) is no longer reachable via `.source()` / `anyhow::Chain` for downstream callers or structured logging.

**Why it matters**: This is the same pattern flagged in sibling provider crates (TASK-1523 in `deps`, ERR-4 family). Operators inspecting a failed `ops about --refresh` lose the underlying error kind — they only see the rendered string. Replace with `anyhow::Error::new(e).context("cargo update --dry-run failed")` (or `Err(e).context(...)?`) so the source chain survives.

**Why it matters**: Lossy error wrapping makes timeouts indistinguishable from spawn failures in logs and in any structured-error consumer downstream of `DataProviderError`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 RunError is wrapped with .context() (or anyhow::Error::new(e).context(...)) so .source() returns the original RunError.
- [ ] #2 A unit test asserts the error chain contains both the 'cargo update --dry-run failed' context and the underlying RunError (e.g. by walking anyhow::Chain or matching on source().is::<RunError>()).
<!-- AC:END -->
