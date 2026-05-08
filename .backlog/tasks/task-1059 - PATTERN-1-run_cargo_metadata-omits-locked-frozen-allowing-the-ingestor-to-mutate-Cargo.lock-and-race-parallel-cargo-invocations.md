---
id: TASK-1059
title: >-
  PATTERN-1: run_cargo_metadata omits --locked / --frozen, allowing the ingestor
  to mutate Cargo.lock and race parallel cargo invocations
status: Done
assignee: []
created_date: '2026-05-07 21:04'
updated_date: '2026-05-08 06:52'
labels:
  - code-review
  - extensions-rust
  - metadata
  - PATTERN-1
  - CONC-2
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
extensions-rust/metadata/src/lib.rs:30-37 invokes `cargo metadata --format-version 1` without --locked or --frozen. cargo metadata, by default, is allowed to update Cargo.lock to refresh resolver state — adding entries for new transitive deps, refreshing yanked-version resolutions, and (with a registry probe enabled) touching the index. Three observable consequences for the ops pipeline:

1. Race conditions in CI: 'ops about' and a parallel 'cargo build' / 'cargo test' running in the same workspace can both attempt to rewrite Cargo.lock, producing intermittent 'Cargo.lock has been modified' churn or a corrupted lockfile under unfortunate interleavings.
2. Silent network/disk side effects from a read-only command: the user invoked 'ops about' (a reporting command) and got an unexpected lockfile rewrite + git diff. Sister providers (cargo-update, cargo-deny) deliberately drop --dry-run modifications; metadata should be at least as conservative.
3. Reproducibility: two consecutive ingestor runs against the same workspace can yield different metadata if cargo refreshes the index between them, breaking the contract that data_sources.checksum is stable for an unchanged workspace.

Fix: pass --locked (or --frozen for stricter behaviour) to cargo metadata. --locked fails fast if the lockfile would need to be updated — surfacing the drift to the operator instead of silently rewriting. --frozen additionally forbids network access. --locked is the conservative default; choose --frozen if the ingestor must be fully offline.

Audit cargo-update and deps cargo invocations for the same flag in a follow-up.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 run_cargo_metadata passes --locked (or --frozen, justified in the comment)
- [x] #2 Regression test asserts the arg list contains the chosen flag
- [x] #3 Audit cargo-update and deps invocations and either update them or document the rationale
<!-- AC:END -->
