---
id: TASK-2590
title: 'API-15: ops-text-fixers library declares no rust-version (MSRV)'
status: To Do
assignee: []
created_date: '2026-10-10 15:46'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - api
dependencies: []
parent_task_id: 'TASK-2608'
modified_files:
  - extensions/text-fixers/Cargo.toml
priority: low
ordinal: 1000
dedup_key: 'API-15:extensions/text-fixers/Cargo.toml:ops-text-fixers'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/text-fixers/Cargo.toml:1-23`

**What**: The crate's manifest inherits `version.workspace`, `edition.workspace` and `license.workspace` but omits `rust-version.workspace = true`, so it declares no MSRV of its own while the workspace package table carries `rust-version = "1.97"` to inherit. Same finding class as ops-theme (TASK-2450), ops-about-python (TASK-2481), ops-cargo-toml (TASK-2495), ops-tfplan (TASK-2504), ops-extension (TASK-2525), ops-rust-loc (TASK-2546), ops-git (TASK-2555) and ops-run-before-push (TASK-2578).

**Why it matters**: API-15 — a library with no declared MSRV has an accidental one that changes with whatever the maintainer happened to compile with. The crate already relies on post-1.80 stabilizations (e.g. `Vec::with_capacity`-era APIs aside, `let ... else` and `is_some_and`), and version-gated review rules (VER-*) are decided by the declared MSRV; leaving it implicit means nothing records which stabilizations the crate may use.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 extensions/text-fixers/Cargo.toml declares rust-version.workspace = true (or an explicit MSRV no lower than the workspace's 1.97)
- [ ] #2 cargo check -p ops-text-fixers succeeds and the workspace MSRV machinery recognizes the crate
<!-- AC:END -->
