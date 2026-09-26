---
id: TASK-2287
title: 'Add ops about machine --json: build-relevant machine state for timing records'
status: To Do
assignee: []
created_date: '2026-09-26 17:47'
updated_date: '2026-09-26 18:27'
labels:
  - feature
  - about
  - skills-integration
dependencies: []
parent_task_id: 'TASK-2292'
modified_files:
  - crates/cli/src/about_cmd.rs
  - extensions/about/src/lib.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**What**: `ops about machine [--json]` reports:

- cores and load average
- other running `cargo`, `rustc` and `cargo-nextest` processes
- the effective `build.jobs`, rustc wrapper, target dir, linker and rustflags across all cargo config layers (repo, parent directories, `~/.cargo/config.toml`, env), each with its source
- the filesystem type and size of `TMPDIR` and of the target dir, flagging tmpfs
- sccache counters, when sccache is the wrapper

It works on Linux and macOS.

**Why**: `rust-make-build-fast` records this next to every timing, because a number without it cannot be compared with the next one. Today it takes `nproc`, `/proc/loadavg`, `pgrep`, `findmnt`, `df`, reading four config layers, and `sccache --show-stats --stats-format=json`, and several of those are Linux-only. Two real hazards live here: a user-level `jobs = 2` cap makes timings measure the cap, and a tmpfs `/tmp` fails cold builds with exit 101 and no compile error.

**Used by**: rust-make-build-fast.
Source: skills-vs-ops audit of rsvalerio/ai dev-skills, 2026-09-26 (https://claude.ai/artifact/4yaKVkPdfe93hrqhFW5u1z).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Reports cores, load, other build processes, effective jobs/wrapper/target-dir/linker with their source layer, and tmpfs status of TMPDIR and target dir
- [ ] #2 Works on Linux and macOS
- [ ] #3 JSON output (`--json`) carries a schemaVersion
<!-- AC:END -->
