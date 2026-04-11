---
id: TASK-0024
title: dir_name utility duplicated across 3 about extensions
status: Triage
assignee: []
created_date: '2026-04-11 18:30:00'
labels:
  - rust-code-duplication
  - CD
  - DUP-3
  - low
  - ext-about
dependencies: []
---

## Description

**Location**: `extensions-java/about/src/lib.rs:448-452`, `extensions-go/about/src/lib.rs:171-175`, `extensions-rust/about/src/identity.rs:99-103`
**Anchor**: `fn dir_name`
**Impact**: The `dir_name` utility function is duplicated across three about-extension crates. All implementations extract the last path component and fall back to `"project"`. The Go and Java versions return `&str`, while the Rust version returns `String` — the logic is otherwise identical. A fourth inline variant exists in `extensions/about/src/lib.rs:115-118` (`build_fallback_identity`).

**Notes**:
Fix options:
1. Move `dir_name` to `ops_core` as a small path utility (e.g., `ops_core::path_util::dir_name_or_default`)
2. Standardize the return type to `&str` (the `&str` variant covers the `String` case via `.to_string()` at the call site)

This is a low-severity pattern issue — the function is only 4 lines. But with 3+ occurrences across independently maintained extension crates, drift is likely (the Rust version already has a different return type).

DUP-3: 3+ occurrences of a repeated utility pattern across modules.
