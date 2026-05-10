---
id: TASK-0585
title: 'ARCH-11: ops-about pins unicode-width = "0.2" outside [workspace.dependencies]'
status: Done
assignee:
  - TASK-0637
created_date: '2026-04-29 05:17'
updated_date: '2026-04-29 06:31'
labels:
  - code-review-rust
  - ARCH
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/Cargo.toml:22`

**What**: Every other dependency in extensions/about/Cargo.toml uses `{ workspace = true }`, but `unicode-width = "0.2"` is pinned literally. TASK-0413 (Done) centralized tempfile/sha2/hex/tokei/duckdb/toml_edit — unicode-width was missed.

**Why it matters**: ARCH-11. A second consumer or version bump diverges silently; trivially fixed by moving to [workspace.dependencies].
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 unicode-width declared once under [workspace.dependencies] in root Cargo.toml
- [ ] #2 extensions/about/Cargo.toml references via { workspace = true }
- [ ] #3 cargo metadata shows single resolved version
<!-- AC:END -->
