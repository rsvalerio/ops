---
id: TASK-2102
title: 'UNSAFE-12: crate is unsafe-free but cannot say so mechanically - no unsafe_code forbid reachable under the centralized lint policy'
status: Done
assignee: []
created_date: '2026-09-08 06:42'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - unsafe
dependencies: []
parent_task_id: 'TASK-2245'
modified_files:
  - crates/backlog/Cargo.toml
priority: low
ordinal: 25000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/backlog/Cargo.toml:22`

**What**: crates/backlog contains no `unsafe` (verified by scan), but neither its `[lints]` table (which only inherits `workspace = true`) nor `[workspace.lints.rust]` in the root Cargo.toml sets `unsafe_code = "forbid"`. Other workspace members (crates/runner, crates/core, extensions/duckdb) do use unsafe, so a workspace-wide forbid is not the drop-in fix, and the project's ARCH-11 policy ("no crate may set its own lint levels in Cargo.toml") currently leaves an unsafe-free member with no way to state the property mechanically.

**Why it matters**: UNSAFE-12: `forbid` turns "does this crate contain unsafe?" into a build-enforced fact instead of a reviewer re-answering it — and it survives the well-meaning PR that adds one `unsafe` block for a micro-optimization. The policy interaction is the real finding: the centralized table needs a per-member escape hatch (e.g. an agreed `[lints.rust] unsafe_code = "forbid"` exception for unsafe-free members, or moving the key into each member's inherited-and-extended lints) so safe crates can opt in.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 crates/backlog builds with unsafe_code = forbid in force (crate-level lints or an agreed workspace-policy exception), without loosening the policy for members that genuinely use unsafe
- [x] #2 Adding an unsafe block to crates/backlog fails the build
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Landed in wave TASK-2245: added `#![forbid(unsafe_code)]` to crates/backlog/src/lib.rs crate root — crate-level enforcement via inner attribute, keeping `[lints] workspace = true` (ARCH-11) intact, so no policy loosening for unsafe-holding members. AC#2 verified by negative probe: appending `unsafe { 1u8 + 1u8 }` to the crate fails cargo check with `error: usage of an unsafe block` under the forbid (probe removed afterwards). The lone "unsafe" token in crates/backlog/src/model.rs:80 is prose in a doc comment, not code.
<!-- SECTION:NOTES:END -->
