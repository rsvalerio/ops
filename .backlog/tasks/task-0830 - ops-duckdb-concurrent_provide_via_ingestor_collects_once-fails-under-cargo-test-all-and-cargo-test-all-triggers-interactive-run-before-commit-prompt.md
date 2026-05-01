---
id: TASK-0830
title: >-
  ops-duckdb concurrent_provide_via_ingestor_collects_once fails under cargo
  test --all and cargo test --all triggers interactive run-before-commit prompt
status: Done
assignee: []
created_date: '2026-05-01 06:49'
labels:
  - code-review-rust
  - tests
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Failure 1 — duckdb test fails on cargo test --all --all-features

`extensions/duckdb/src/sql/ingest.rs` — `sql::ingest::tests::concurrent_provide_via_ingestor_collects_once` panics:

```
thread '...' panicked at extensions/duckdb/src/sql/ingest.rs:728:36:
ingest 1: SQL validation failed: invalid character in path: ':'

Caused by:
    invalid character in path: ':'
```

Reproduces on clean `main` (git stash + rerun). The test appears to use `:memory:` as a path component, which the SQL path validator now rejects. Side effect: the test leaves a stray `extensions/duckdb/:memory:.ingest/` directory in the repo root that has to be deleted manually.

**What to do:**
- Decide whether the validator should permit `:memory:` (DuckDB's in-memory sentinel) or whether the test should switch to a tempdir-backed path so the validator gate stays strict.
- Ensure no on-disk directory is created for the in-memory path.

## Failure 2 — cargo test --all triggers interactive prompt

Running `cargo test --all --all-features` from the workspace root prints:

```
ops: note: no 'run-before-commit' command configured in .ops.toml.
? Run `ops run-before-commit install` now? (Y/n)
```

A test or build script is invoking ops/git-hook machinery and blocking on a TTY prompt. This breaks non-interactive CI/cargo runs.

**What to do:**
- Locate the call site (likely a test or build.rs in extensions/git or hook-common).
- Either skip the prompt under `CI`/non-tty/`OPS_NONINTERACTIVE`, or stop running interactive ops machinery from inside cargo test.
<!-- SECTION:DESCRIPTION:END -->
