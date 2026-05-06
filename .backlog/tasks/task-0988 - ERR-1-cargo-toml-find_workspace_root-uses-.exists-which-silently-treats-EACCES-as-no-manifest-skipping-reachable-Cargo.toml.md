---
id: TASK-0988
title: >-
  ERR-1: cargo-toml find_workspace_root uses .exists() which silently treats
  EACCES as 'no manifest', skipping reachable Cargo.toml
status: To Do
assignee:
  - TASK-1013
created_date: '2026-05-04 21:59'
updated_date: '2026-05-06 06:48'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/lib.rs:351` (find_workspace_root)

**What**: The ancestor walk gates each candidate via `cargo_toml.exists()`. `Path::exists()` is documented to return `false` on any IO error (PermissionDenied, EIO, EBUSY, …) — it cannot distinguish "file is absent" from "I'm not allowed to stat the parent". A workspace root manifest sitting in a directory the user can traverse but not stat (uncommon but real on multi-tenant / shared-CI setups, NFS root_squash, sandboxed containers with restricted CWD) will be silently skipped, and the walk will continue upward until it finds a member-only manifest or hits MAX_ANCESTOR_DEPTH and returns NotFound.

**Why it matters**: The result is a misrooted workspace (the entire about/units/coverage stack scopes to a member crate) or a spurious "no manifest" error, both with zero log evidence. The neighbouring `manifest_declares_workspace` already handles `Err(NotFound) => return false` and warns on other IO errors — `find_workspace_root` should mirror that and use `try_exists()` (or `metadata().map(|_| true)`) so a stat error surfaces as either `CanonicalizeFailed`-style typed variant or a tracing breadcrumb instead of silently masquerading as absent.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace cargo_toml.exists() with try_exists() (or std::fs::metadata) so non-NotFound stat errors are distinguished from genuine absence
- [ ] #2 Non-NotFound stat errors emit a tracing::warn breadcrumb (path Debug-formatted per the TASK-0947 policy) and the walk's behaviour for the affected ancestor is documented (skip-and-continue vs hard-fail)
<!-- AC:END -->
