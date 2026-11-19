---
id: TASK-2738
title: 'TEST-33: platform_filter reads $RUSTC and spawns rustc with no injection seam; probe-failure paths untested'
status: To Do
assignee: []
created_date: '2026-11-18 18:38'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - TEST
dependencies: []
parent_task_id: 'TASK-2849'
modified_files:
  - extensions-rust/about/src/deps_provider.rs
priority: low
ordinal: 1000
dedup_key: 'TEST-33:extensions-rust/about/src/deps_provider.rs:platform_filter'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/deps_provider.rs:164` (`platform_filter`), `extensions-rust/about/src/deps_provider.rs:210` (`rustc_stdout`)

**What**: `platform_filter` reads `std::env::var_os("RUSTC")` and runs `rustc -vV` / `rustc --print cfg --target <t>` directly. Unlike `find_duplicates`, which takes the `cargo update` dry-run as an injected `updater` closure, the rustc probe has no seam, so the only tested path is `--target all` (which skips rustc). The documented error contract (`ComputationMessage` on a failed probe or unknown triple instead of widening to every platform), the "no host triple" branch, and the host-reuse branch for a requested triple equal to the host are not exercised.

**Why it matters**: The error path is the one that protects the duplicates report from silently widening; it can regress unnoticed, and testing it today would require mutating process env (`RUSTC`) in a parallel test suite.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The rustc probe is injectable (e.g. a probe closure or a rustc path parameter resolved once in provide), with RUSTC read only at the boundary
- [ ] #2 Tests cover a failing probe (error returned, no PlatformFilter::All fallback), a missing host triple, and a requested triple equal to the host
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.11 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
