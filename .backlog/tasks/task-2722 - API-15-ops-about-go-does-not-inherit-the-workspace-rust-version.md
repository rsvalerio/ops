---
id: TASK-2722
title: 'API-15: ops-about-go does not inherit the workspace rust-version'
status: To Do
assignee: []
created_date: '2026-11-18 18:33'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - API
dependencies: []
parent_task_id: 'TASK-2847'
modified_files:
  - extensions-go/about/Cargo.toml
priority: low
ordinal: 1000
dedup_key: 'review-2667.07-11'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/Cargo.toml:1`

**What**: The `[package]` table sets `version.workspace`, `license.workspace` and a literal `edition = "2024"` but omits `rust-version.workspace = true`, so the crate has no declared MSRV although the workspace defines `rust-version = "1.97"` and sibling crates (e.g. `extensions-python/about`) inherit it. The crate relies on let-chains (modules.rs:180) and `Option::is_none_or`, which have minimum compiler versions.

**Why it matters**: Without a declared MSRV, `cargo` cannot reject an older toolchain with a clear message, and MSRV-gated rules (API-15) have nothing to check against for this crate. Note: ~22 of 32 workspace crates share this gap; this task covers ops-about-go only.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 extensions-go/about/Cargo.toml declares rust-version.workspace = true
- [ ] #2 cargo metadata reports rust_version 1.97 for ops-about-go
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.07 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
