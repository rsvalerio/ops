---
id: TASK-2069
title: >-
  DUP-3: six more hand-rolled tracing-capture scaffolds outside the TASK-2058
  enumeration
status: Triage
assignee: []
created_date: '2026-08-29 18:21'
labels:
  - code-review-rust
  - duplication
dependencies: []
modified_files:
  - crates/runner/src/command/tests/parallel.rs
  - crates/runner/src/command/tests/expand.rs
  - crates/extension/src/tests.rs
  - extensions/run-before-commit/src/lib.rs
  - extensions/hook-common/src/git_state.rs
  - extensions/hook-common/src/git.rs
  - extensions/duckdb/src/sql/ingest/orchestrator.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/tests/parallel.rs:515`, `crates/runner/src/command/tests/expand.rs:133`, `crates/extension/src/tests.rs:844`, `extensions/run-before-commit/src/lib.rs:818`, `extensions/hook-common/src/git_state.rs:338`, `extensions/hook-common/src/git.rs:551`, `extensions/duckdb/src/sql/ingest/orchestrator.rs:473`

**What**: TASK-2058 enumerated seven `TracingBuf` consumers and they are now on
`ops_core::test_utils::capture_tracing`. These sites were outside that
enumeration and still open-code the same scaffold: a private `VecWriter` /
`BufWriter` + `MakeWriter` shim (or, in `parallel.rs`, a hand-built
`tracing::Dispatch`), a `tracing_subscriber::fmt()` builder repeating
`with_max_level` / `with_ansi(false)`, and `with_default`. None of them pins
the global dispatcher.

`extensions-rust/about/src/coverage_provider.rs` is deliberately *not* in this
list: it installs per-thread subscribers over a shared buffer, which
`capture_tracing` cannot express, and it pins the dispatcher itself.

**Why it matters**: DUP-3, and the same silent-flake class the shared harness
exists to close — without a pinned global dispatcher, a parallel first hit can
cache `Interest::never()` for the warn callsite and the capture comes back
empty at random. Each copy also picks its own subscriber configuration, so
"captured output" means something slightly different per crate.

**Origin**: discovered during TASK-2061 while fixing TASK-2058.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each listed site captures through ops_core::test_utils::capture_tracing (or capture_warn) instead of a private writer + fmt() builder
- [ ] #2 The private VecWriter/BufWriter MakeWriter shims those sites owned are deleted, with no remaining users
- [ ] #3 Assertions at each site are unchanged and the suites stay green under cargo nextest
<!-- AC:END -->
