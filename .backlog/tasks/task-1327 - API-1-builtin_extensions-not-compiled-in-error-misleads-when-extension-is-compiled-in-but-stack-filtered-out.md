---
id: TASK-1327
title: >-
  API-1: builtin_extensions 'not compiled in' error misleads when extension is
  compiled-in but stack-filtered out
status: Done
assignee:
  - TASK-1383
created_date: '2026-05-12 16:21'
updated_date: '2026-05-12 23:16'
labels:
  - code-review-rust
  - api
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/registry/discovery.rs:118-131`

**What**: `builtin_extensions` filters compiled-in extensions by `ext.stack()` against the detected stack *before* checking `config.extensions.enabled`. A stack-specific extension (e.g. `stack-rust` extensions like `cargo_toml`, `cargo_update`, `metadata`) that is compiled into the binary but whose `stack()` does not match the current project will be dropped from `available` at line 97-103. If the user then has that name in `[extensions] enabled = [...]`, the loop at line 118 bails with `"extension '{name}' enabled in config but not compiled in"` — but it *is* compiled in; it was just filtered out by the stack check.

**Why it matters**: Operator-facing error misleads. A user who copied an `extensions.enabled = ["cargo_toml"]` line into a Node/Go project sees "not compiled in" and may rebuild with different features, edit Cargo.toml, or report a packaging bug — when the actual fix is to remove the entry or run in the matching stack. The error claims a packaging fault that does not exist.

The `available` BTreeMap loses the distinction between "factory declined / not linked" and "compiled-in but stack-filtered". The bail path should differentiate, e.g. by checking the unfiltered `collect_compiled_extensions` result and reporting "compiled in but disabled for the current stack ({detected_stack})" when applicable.

Repro shape: Build `ops` with `--features stack-rust`, run `ops extension list` in a directory with only `package.json` (stack=Node) and an `.ops.toml` containing `[extensions] enabled = ["cargo_toml"]`. Observe the misleading message.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 builtin_extensions distinguishes 'not compiled in' from 'compiled in but stack-filtered'
- [ ] #2 the error names the detected stack when the extension was stack-filtered
- [ ] #3 regression test exercises stack-rust extension name listed in extensions.enabled on a non-Rust stack and asserts on the precise wording
<!-- AC:END -->
