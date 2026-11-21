---
id: TASK-2896
title: 'API-14: no verify gate builds public-only rustdoc, so private intra-doc links can regress'
status: Done
assignee: []
created_date: '2026-11-20 18:24'
updated_date: '2026-11-20 21:30'
labels:
  - code-review-rust
  - api
dependencies: []
parent_task_id: 'TASK-2897'
modified_files:
  - .ops.toml
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `.ops.toml:36`

**What**: `ops verify` documents the workspace with `--document-private-items`, and the per-crate `cargo rustdoc` gates cover only ops-text-fixers and ops-config-checkers. Nothing builds public-only docs for ops-about, ops-extension or ops-rust-foundation, whose private intra-doc links were just cleared.

**Why it matters**: A new public doc linking a private item compiles cleanly under the gate and ships a dead link in published docs; the cleanup will drift back.

**Origin**: discovered during TASK-2891 while fixing TASK-2889.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 ops verify fails when a public doc in ops-about, ops-extension or ops-rust-foundation links a private item

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Added public-docs-{about,extension,rust-foundation} leaves to .ops.toml (cargo rustdoc -p <crate> --lib --all-features -- -D warnings, no --document-private-items) aggregated by public-docs, appended to verify. Verified the gate fails when a public doc links a private item (temp probe on ops-extension, reverted) and passes clean for all three crates; ops verify 14/14.
<!-- SECTION:NOTES:END -->
