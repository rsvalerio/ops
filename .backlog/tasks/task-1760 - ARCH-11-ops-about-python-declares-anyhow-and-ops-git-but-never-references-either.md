---
id: TASK-1760
title: >-
  ARCH-11: ops-about-python declares anyhow and ops-git but never references
  either
status: Triage
assignee: []
created_date: '2026-08-27 11:19'
labels:
  - code-review-rust
  - architecture
dependencies: []
modified_files:
  - extensions-python/about/Cargo.toml
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-python/about/Cargo.toml:11-16`

**What**: The crate declares two dependencies with no use anywhere under `extensions-python/about/src/`:

- `anyhow` (line 16) — the crate's error type is `ops_extension::DataProviderError` throughout; `anyhow` appears in neither `lib.rs` nor `units.rs`.
- `ops-git` (line 12) — the git-remote repository fallback the crate relies on happens inside `ops_about::identity::build_identity_value`, which calls `ops_git::resolve_repository_with_git_fallback` through **its own** dependency. The test `git_remote_fallback_when_no_repository_url` (`lib.rs:875`) exercises that path without this crate naming `ops-git` directly.

Verified with `grep -rn "anyhow\|ops_git" extensions-python/about/src/` — no matches.

**Why it matters**: unused dependencies inflate the build graph and the `cargo audit` / `ops sec` surface for every consumer of the workspace, and they mislead the next reader about where the crate's error handling and git access come from. Removing `ops-git` in particular makes the real ownership visible: the git fallback is `ops_about`'s contract, not this crate's.

**Note**: the same class of finding was filed this run against the Go and Java about crates (TASK-1738, TASK-1749) — both name `anyhow` and `ops-git` for the same reason, which suggests these three were copied from a common template. Worth sweeping the remaining `extensions-*/about/Cargo.toml` files when this is fixed.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 anyhow and ops-git are removed from extensions-python/about/Cargo.toml
- [ ] #2 cargo build --all-targets and the full test suite pass unchanged
- [ ] #3 The remaining extensions-*/about/Cargo.toml files are checked for the same two unused entries
<!-- AC:END -->
