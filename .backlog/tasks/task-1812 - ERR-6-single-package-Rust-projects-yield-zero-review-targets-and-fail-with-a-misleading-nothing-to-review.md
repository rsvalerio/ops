---
id: TASK-1812
title: >-
  ERR-6: single-package Rust projects yield zero review targets and fail with a
  misleading 'nothing to review'
status: To Do
assignee:
  - TASK-1996
created_date: '2026-08-27 11:32'
updated_date: '2026-08-28 14:13'
labels:
  - code-review-rust
  - idioms-correctness
dependencies: []
modified_files:
  - extensions-rust/create-review-tasks/src/provider.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/create-review-tasks/src/provider.rs:21-47`

**What**: `provide` resolves the root with `find_workspace_root_strict`, then derives targets solely from `ops_about_rust::resolved_workspace_members(&manifest, &root)`. That helper returns `Vec::new()` as soon as the manifest has no `[workspace]` table (`extensions-rust/about/src/query.rs:512-514`), and `find_workspace_root_strict` deliberately falls back to the first `Cargo.toml` it finds even when that manifest declares no workspace (`extensions-rust/cargo-toml/src/workspace_root.rs`, `CandidateAction::RecordFirst` + the `if let Some(root) = first_cargo_toml` tail of `walk_ancestors`).

So for an ordinary single-package Rust project — no `[workspace]` table, the most common Rust project shape after this workspace itself — the provider succeeds and returns `{"skill": "code-review-rust", "targets": []}`. The generic engine then hard-errors on the empty list: `extensions/create-review-tasks/src/lib.rs:113-116` reports `review_targets provider returned no targets — nothing to create review tasks for`. The operator is told there is nothing to review, when in fact there is exactly one crate to review and the provider simply has no code path that emits it.

The crate's own test suite states the intended invariant — `missing_workspace_manifest_is_an_error` (provider.rs:177-189) documents that an empty target list "the engine would misread as 'nothing to review'" and must never be returned in place of a real condition — but only covers the "no Cargo.toml at all" case. The "Cargo.toml exists, has `[package]`, has no `[workspace]`" case falls straight through the gap the test comment warns about: the empty `Vec` is used as a sentinel that the caller decodes as a different condition (ERR-6).

**Why it matters**: `ops create-review-tasks` is unusable on any non-workspace Rust project, and the failure mode is a misleading message that points the operator at the backlog rather than at the real cause. The fix is a decision, not just a message: either emit the root package itself as the single review target (matching what a reviewer would expect), or return a typed error that names the actual condition ("manifest at <root> declares no [workspace]; create-review-tasks needs at least one review target").
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 provide() no longer returns an empty targets list for a manifest that parses but declares no [workspace] table
- [ ] #2 Decide and document the chosen behaviour: either the root [package] becomes the single review target, or a typed error naming the missing [workspace] table is returned
- [ ] #3 A test builds a scratch single-package project (Cargo.toml with [package] and no [workspace]) and asserts the chosen behaviour end to end
- [ ] #4 The doc comment on provide/the module states which project shapes are supported
<!-- AC:END -->
