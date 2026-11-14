---
id: TASK-2357
title: 'UNSAFE-12: ops-about-python is unsafe-free but does not forbid unsafe code mechanically'
status: Done
assignee: []
created_date: '2026-10-04 14:09'
updated_date: '2026-10-04 14:31'
labels:
  - code-review-rust
  - unsafe
dependencies: []
modified_files:
  - extensions-python/about/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'UNSAFE-12:extensions-python/about/src/lib.rs:crate'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-python/about/Cargo.toml`, `extensions-python/about/src/lib.rs:1`

**What**: The crate (lib.rs 1355 lines, units.rs 489 lines) contains no `unsafe`, and inherits `[workspace.lints]`, but the root `[workspace.lints.rust]` table (Cargo.toml:100-103) sets no `unsafe_code` lint, and the crate root carries no `#![forbid(unsafe_code)]`.

**Why it matters**: A later PR can add an `unsafe` block with no build-time signal; `forbid` makes "this crate has no unsafe" an enforced fact (UNSAFE-12).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Crate root has #![forbid(unsafe_code)] or unsafe_code = "forbid" is inherited via workspace lints
- [ ] #2 cargo build and clippy pass for ops-about-python
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Superseded by TASK-2368 (workspace-wide unsafe_code lint). Closed as a duplicate; no code change.
<!-- SECTION:NOTES:END -->
