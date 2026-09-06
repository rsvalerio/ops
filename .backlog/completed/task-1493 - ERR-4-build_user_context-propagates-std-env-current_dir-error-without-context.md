---
id: TASK-1493
title: >-
  ERR-4: build_user_context propagates std::env::current_dir error without
  context
status: Done
assignee:
  - TASK-1647
created_date: '2026-05-18 17:28'
updated_date: '2026-05-25 18:53'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/lib.rs:176-180`

**What**: `build_user_context` does `let cwd = std::env::current_dir()?;` then constructs the `Context`. The `?` propagates the raw `std::io::Error` (typically `ENOENT` on a deleted-cwd, `EACCES` on a permission flip). With no `.with_context()`, the operator sees a bare `No such file or directory (os error 2)` in the failure path of `run_deps` — they cannot tell whether the missing path is the user's cwd, `.ops.toml`, the cargo workspace, or something inside cargo-deny.

`run_deps` itself adds no context layer either — the error bubbles straight through `anyhow::Result<()>` to the top-level CLI handler.

**Why it matters**: ERR-4 requires `.with_context()` on every `?` whose source error does not by itself identify the failing operation. A bare io::Error 2 in a deps-gate failure path is the canonical "what is even failing" debug-loop signal — it costs nothing to attach "while resolving current working directory for ops deps" at the source.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 build_user_context wraps current_dir() with anyhow::Context describing the operation (e.g. 'reading current working directory for deps command')
- [ ] #2 No behavioural change on the happy path; existing tests pass
<!-- AC:END -->
