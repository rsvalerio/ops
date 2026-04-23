---
id: TASK-0145
title: >-
  DUP-3: hook_config() boilerplate duplicated across run-before-commit and
  run-before-push
status: Done
assignee: []
created_date: '2026-04-22 21:22'
updated_date: '2026-04-23 07:39'
labels:
  - rust-code-review
  - DUP
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Files**:
- `extensions/run-before-commit/src/lib.rs:35`
- `extensions/run-before-push/src/lib.rs:35`

**What**: Both crates define a near-identical `hook_config()` constructor plus thin wrapper fns (`should_skip`, `find_git_dir`, `install_hook`, `ensure_config_command`) that all forward to `ops_hook_common::*` passing `&hook_config()`. The only differences are the constants (name, filename, script, env var, legacy markers, help). The hook-common crate already owns a `HookConfig` struct; the per-crate wrappers exist only to pre-bind those constants.

**Why it matters**: Adding a new hook kind (e.g. `post-merge`) or changing the wrapper signatures requires parallel edits in both crates. The constants could live as `pub const HOOK_CONFIG: HookConfig = HookConfig { ... }` with wrapper fns auto-generated via a small macro, or the crates could expose only the constants and let callers use `ops_hook_common` directly.

**Why it matters**: Structural duplication across sibling crates; low churn today but bloats the surface that must be kept in sync with `HookConfig`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Eliminate wrapper-fn duplication via const HookConfig or a shared macro
- [x] #2 Wrappers in both crates stay byte-for-byte generated from a single source of truth
<!-- AC:END -->
