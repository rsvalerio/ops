---
id: TASK-1772
title: >-
  ERR-1: find_required_version's .flatten() silently drops per-entry read_dir
  errors, contradicting the module's own IO policy
status: Done
assignee:
  - TASK-2001
created_date: '2026-08-27 11:21'
updated_date: '2026-08-28 21:24'
labels:
  - code-review-rust
  - idioms-correctness
dependencies: []
modified_files:
  - extensions-terraform/about/src/lib.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/about/src/lib.rs:124-136` (`find_required_version`), related sites at `:414-415` (`count_local_modules`)

**What**: The module docs (`:6-15`) state the crate-wide rule: "missing manifest is silent, real IO error is `tracing::warn!`-and-fall-back", and say the directory enumeration in `find_required_version` "mirrors the same policy". The `read_dir` *call* does honour it (`:105-116`), but the iteration immediately discards it:

```rust
let mut tf_paths: Vec<std::path::PathBuf> = entries
    .flatten()
    .map(|e| e.path())
```

`.flatten()` on `ReadDir` drops every `Err(io::Error)` entry — permission denied on a subentry, EIO, a vanished dirent — with no log at all. `count_local_modules` was fixed for exactly this at `:402-413`, and its doc comment explicitly calls out "Per-entry `read_dir` failures are similarly logged rather than silently dropped via `flatten()`" — so the two sibling functions in the same file now implement opposite policies.

Two smaller variants of the same swallow remain inside `count_local_modules` itself: `.filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))` (`:414`) and `.filter(|e| e.path().join("main.tf").exists())` (`:415`) both collapse an `io::Error` into "not a module", so an unreadable `modules/<name>/` is reported as absent rather than warned about. (`Path::exists()` also swallows non-NotFound errors by contract.)

**Why it matters**: The whole point of the TASK-0851/TASK-1018 policy is that an operator running `ops about` in a directory they cannot fully read gets told so instead of quietly seeing "no version declared" / "no modules". The `flatten()` here reintroduces the silent-degradation path the policy exists to close, and because the two functions sit twenty lines apart the inconsistency is a live drift surface for the next stack that copies this file.

**Fix direction**: mirror `count_local_modules`' `filter_map` + `tracing::warn!` shape in `find_required_version`; for the `file_type()` / `exists()` filters, match on the error and warn on non-NotFound rather than folding it into `false`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 find_required_version logs a tracing::warn! for each failed read_dir entry instead of dropping it via .flatten()
- [x] #2 count_local_modules warns on non-NotFound errors from file_type() and the main.tf existence probe rather than treating them as "not a module"
- [x] #3 A test asserts the warn path for count_local_modules on a non-NotFound modules/ failure, mirroring find_required_version_warns_when_versions_tf_is_a_directory
<!-- AC:END -->
