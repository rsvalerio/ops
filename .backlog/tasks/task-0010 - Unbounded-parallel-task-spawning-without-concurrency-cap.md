---
id: TASK-0010
title: Unbounded parallel task spawning without concurrency cap
status: Done
assignee: []
created_date: '2026-04-10 12:00:00'
updated_date: '2026-04-11 09:55'
labels:
  - rust-security
  - RS
  - SEC-33
  - medium
  - crate-runner
dependencies: []
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Location**: `crates/runner/src/command/mod.rs:401-423`
**Anchor**: `spawn_parallel_tasks`
**Impact**: `spawn_parallel_tasks` spawns all parallel commands simultaneously with no concurrency cap. A config with hundreds of parallel commands could exhaust file descriptors, process table entries, or memory. The bounded event channel (capacity 256) provides backpressure on event emission but does not limit the number of spawned tasks.

**Notes**:
SEC-33 requires bounding resource consumption. The code acknowledges this in the comment at line 392-400: "If memory becomes a concern with very large parallel groups, consider splitting into smaller batches or adding a config option for maximum parallelism."

Current mitigations:
- Bounded `mpsc::channel(256)` for event backpressure
- `kill_on_drop(true)` on child processes
- `abort` flag for fail-fast cancellation

Missing:
- No cap on `join_set.spawn()` call count — all steps are spawned immediately
- Each spawned task holds a cloned `tx` sender, `abort` Arc, and `cwd` PathBuf
- Each task spawns a child process via `Command::new()` which consumes a file descriptor per pipe (stdin/stdout/stderr × N tasks)

Fix: Add a configurable `max_parallel` option (default: e.g., `num_cpus` or a fixed cap like 32). Batch tasks: spawn up to `max_parallel` at a time, collect results, then spawn the next batch. Alternatively, use a semaphore (`tokio::sync::Semaphore`) to limit concurrent task starts.

**OWASP**: A04 (Insecure Design)
<!-- SECTION:DESCRIPTION:END -->
