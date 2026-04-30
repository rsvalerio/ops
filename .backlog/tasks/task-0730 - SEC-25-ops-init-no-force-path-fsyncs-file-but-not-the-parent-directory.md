---
id: TASK-0730
title: 'SEC-25: ops init no-force path fsyncs file but not the parent directory'
status: To Do
assignee:
  - TASK-0739
created_date: '2026-04-30 05:49'
updated_date: '2026-04-30 06:07'
labels:
  - code-review-rust
  - security
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/init_cmd.rs:62`

**What**: `write_init(force=false)` opens with `OpenOptions::create_new(true)`, writes the body, calls `f.sync_all()`, and returns. The parent directory entry that names `.ops.toml` is not fsynced. The `--force` path delegates to `ops_core::config::atomic_write`, which (per the comment at `crates/core/src/config/edit.rs:73`) does fsync the parent on Unix.

**Why it matters**: A crash or power loss between the file fsync and the next `sync(2)` can lose the new directory entry on ext4/xfs, leaving the user without the `.ops.toml` they think `ops init` just created. The fact that the same crate already has an `atomic_write` helper that closes this gap on the `--force` branch makes the asymmetry the loud bug — the no-force path is the *common* case (first run in a clean repo).

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 write_init no-force path fsyncs the parent directory after create_new succeeds
- [ ] #2 Behaviour parity check: a regression test asserts both --force and no-force paths produce a .ops.toml that survives a simulated fsync-only-the-file failure mode (or, at minimum, both paths pass through the same atomic helper)
<!-- AC:END -->
