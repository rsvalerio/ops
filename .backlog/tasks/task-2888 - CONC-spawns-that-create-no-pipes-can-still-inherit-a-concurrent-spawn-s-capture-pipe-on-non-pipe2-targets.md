---
id: TASK-2888
title: 'CONC: spawns that create no pipes can still inherit a concurrent spawn''s capture pipe on non-pipe2 targets'
status: Done
assignee: []
created_date: '2026-11-19 08:12'
updated_date: '2026-11-20 18:35'
labels:
  - code-review-rust
  - CONC
dependencies: []
parent_task_id: 'TASK-2895'
modified_files:
  - crates/core/src/subprocess/spawn.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/subprocess/spawn.rs`

**What**: `spawn_locked` is taken only by spawns that create pipes. On targets without `pipe2` (macOS), the leak needs two things: one thread is between `pipe()` and `FD_CLOEXEC`, and another thread forks during that window. The forking thread inherits the write end whether or not it creates pipes itself. A `status()` or inherited-stdio `spawn()` that runs while a locked piped spawn is in flight can still be the holder. Today no such spawn runs in the same process at the same time as a captured spawn: raw steps run sequentially, and extension `status()` calls belong to separate subcommands. Nothing enforces that, though.

**Why it matters**: If a future change runs an unlocked spawn alongside captured steps, the delayed-EOF leak from macOS comes back, and it surfaces as "captured output still open after killing the step's process group".

**Origin**: discovered during TASK-2880 while fixing TASK-2867.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Every spawn that can run while a captured spawn is in flight takes spawn_locked, or a test or lint keeps unlocked spawns from appearing on those paths

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Every spawn in the workspace (std and tokio, production and test code) now goes through ops_core::subprocess::spawn_locked via the new spawn_child_locked / status_locked helpers or output_locked; raw steps use spawn_locked_on_pool in the runner. clippy.toml disallowed-methods bans bare std/tokio Command::spawn/output/status, so a new unlocked spawn fails the clippy gate; the two remaining direct tokio spawns sit inside spawn_locked with a reasoned expect. nextest 4085/4085.
<!-- SECTION:NOTES:END -->
