---
id: TASK-2310
title: 'ops about dependencies --duplicates: filter by target platform'
status: To Do
assignee: []
created_date: '2026-09-27 15:20'
updated_date: '2026-09-27 15:25'
labels:
  - feature
  - about
  - deps
  - skills-integration
dependencies: []
parent_task_id: 'TASK-2314'
modified_files:
  - extensions/about/src/deps.rs
  - extensions-rust/about/src/deps_provider.rs
priority: medium
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**What**: `ops about dependencies --duplicates --json` lists every crate with two or more distinct versions in the resolve, whatever platform it compiles for. On a Linux host, about 12 of ops' 27 entries are Windows-, UEFI- or WASI-only (`windows-sys`, `windows_*` ×7, `r-efi`, `wasi`), and none of them is ever compiled here. dbsec shows the same pattern.

A related symptom: `windows-sys` 0.48.0 is listed with an empty `pulledBy`, so the report cannot say what pulls it in. That is most likely a `cfg(target)`-gated edge the puller resolution does not follow.

**Why it matters**: `rust-make-build-fast` (rsvalerio/ai) files DEP-1 findings from this report, and has to tell the agent to discount platform-only entries by hand. It also takes a name-based guess at which crates are Windows-only. A filter in ops would make the report mean "duplicates this build compiles".

**Sketch**: mirror `cargo tree --target`. Default to the host triple, and accept `--target <triple>` (repeatable) and `--target all` for today's behaviour. Resolve `pulledBy` through target-gated edges, so no entry has an empty puller list.

Source: re-eval of `rust-make-build-fast` on ops @ d3a50ef7 and dbsec, 2026-09-27.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 By default --duplicates uses the host target and leaves out crates only reachable through other targets' cfg edges
- [ ] #2 A --target <triple> option selects other targets, and --target all keeps today's behaviour
- [ ] #3 Every listed older version has a non-empty pulledBy, including ones reached through cfg(target) edges
<!-- AC:END -->
