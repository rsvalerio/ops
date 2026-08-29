---
id: TASK-1760
title: >-
  ARCH-11: ops-about-python declares anyhow and ops-git but never references
  either
status: Done
assignee:
  - TASK-1992
created_date: '2026-08-27 11:19'
updated_date: '2026-08-28 20:05'
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
- [x] #1 anyhow and ops-git are removed from extensions-python/about/Cargo.toml
- [x] #2 cargo build --all-targets and the full test suite pass unchanged
- [x] #3 The remaining extensions-*/about/Cargo.toml files are checked for the same two unused entries
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
`anyhow` and `ops-git` removed from `extensions-python/about/Cargo.toml`;
`Cargo.lock` updated. Full workspace `ops verify` (fmt, clippy -D warnings,
build --all-features --all-targets) passes, as does the crate's test suite.

`tracing-subscriber` was *added* as a dev-dependency in the same file — it is
genuinely used, by the TASK-1756 / TASK-1757 warn-capture tests, matching
`extensions-rust/about` and `extensions-terraform/about`.

AC#3 sweep of the remaining `extensions-*/about/Cargo.toml`
(grep for `anyhow` / `ops_git` under each crate's `src/`):
- extensions-go, extensions-java — already clean (TASK-1738, TASK-1749 Done).
- extensions-rust/about — both genuinely used (query.rs, identity/resolver.rs).
- extensions/about — both genuinely used.
- extensions-node/about — `anyhow` unused; already tracked by TASK-2013.
- extensions-terraform/about — both unused; already tracked by TASK-1786.
No new task filed: every remaining offender is already in the backlog.
<!-- SECTION:NOTES:END -->
