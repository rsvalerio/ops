---
id: TASK-0294
title: >-
  ARCH-1: extensions/hook-common/src/lib.rs is ~761 lines mixing git discovery,
  hook install, and .ops.toml edits
status: Done
assignee: []
created_date: '2026-04-23 16:54'
updated_date: '2026-04-23 19:59'
labels:
  - rust-code-review
  - architecture
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/lib.rs:1-761`

**What**: `hook-common` lib.rs is a single 761-line file containing: macro definitions, git-dir discovery (find_git_dir, canonical_git_dir), hook installation (install_hook), toml_edit-based .ops.toml mutation (ensure_config_command), and a very large `#[cfg(test)]` module. These concerns share only the top-level hook feature.

**Why it matters**: ARCH-1 flags modules >500 lines and ARCH-8 requires lib.rs to stay a thin entry point. The git-dir logic has security implications (SEC-14 tasks 0231/0252 already touched it) and deserves its own module so security-relevant changes are isolated and individually testable.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 lib.rs reduced to module declarations, re-exports, and crate-level docs
- [x] #2 Git-dir discovery, hook installation, and .ops.toml editing each live in their own submodule (e.g. git.rs, install.rs, config.rs)
- [x] #3 Test module split to mirror the production module split; cargo test still passes
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Split extensions/hook-common/src/lib.rs (761 lines) into 4 modules: lib.rs (now ~175 lines: HookConfig, should_skip, macro), git.rs (find_git_dir + worktree pointer), install.rs (install_hook + canonicalization guards), config.rs (ensure_config_command). Tests split to mirror production modules. All 21 tests pass, workspace builds clean, clippy clean.
<!-- SECTION:FINAL_SUMMARY:END -->
