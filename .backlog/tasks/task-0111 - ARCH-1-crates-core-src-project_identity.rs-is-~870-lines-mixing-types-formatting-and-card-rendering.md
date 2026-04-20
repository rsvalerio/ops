---
id: TASK-0111
title: >-
  ARCH-1: crates/core/src/project_identity.rs is ~870 lines mixing types,
  formatting, and card rendering
status: Done
assignee: []
created_date: '2026-04-19 18:36'
updated_date: '2026-04-19 19:14'
labels:
  - rust-code-review
  - architecture
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/project_identity.rs` (870 lines)

**What**: Module combines `ProjectIdentity` data types, language/emoji formatting helpers, multi-line value composition, and AboutCard rendering in a single file.

**Why it matters**: ARCH-1 red flags — >500 lines, multiple concerns (data model vs. presentation), and a large public surface. Splitting along the type/format/render seam would clarify dependency direction and let tests target each concern in isolation.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Split rendering/formatting helpers into a submodule (e.g., project_identity/format.rs or project_identity/card.rs)
- [x] #2 Public API re-exports preserved from lib.rs
- [x] #3 cargo fmt; cargo clippy --all-targets -- -D warnings; cargo test all pass
<!-- AC:END -->
