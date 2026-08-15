---
id: TASK-0165
title: >-
  ARCH-1: crates/core/src/config/mod.rs mixes Config, overlays, CommandSpec,
  CommandId and init_template in 512 lines
status: To Do
assignee: []
created_date: '2026-04-22 21:24'
updated_date: '2026-08-15 00:00'
labels:
  - rust-code-review
  - ARCH
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: crates/core/src/config/mod.rs:1-512

**What**: The module owns at least five unrelated concerns in one file: (1) Root Config + validate + resolve_alias (25-68); (2) Sub-config structs ExtensionConfig, AboutConfig, DataConfig, OutputConfig + their overlays (70-214); (3) CommandSpec + ExecCommandSpec + CompositeCommandSpec (242-370); (4) CommandId newtype with 10+ hand-written trait impls — Deref, AsRef, Borrow, Display, From<String>, From<&str>, PartialEq<str>, PartialEq<&str>, PartialEq<String> (372-446); (5) default_ops_toml, InitSections, init_template — CLI init logic (448-509).

**Why it matters**: ARCH-1 / ARCH-3. 512 lines is past the 500-line red flag and the concerns are independently evolvable. CommandId alone is a self-contained newtype that belongs in its own file; init_template is CLI-shaped logic that belongs in an init module; the Config*Overlay types mirror the live types and would benefit from co-location with merge.rs. Splitting reduces rebuild surface and makes each concern easier to test.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract CommandId + impls to its own module (config/command_id.rs)
- [ ] #2 Extract InitSections and init_template to config/init.rs
- [ ] #3 Keep mod.rs as a thin re-export hub (<150 lines)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Deferred: this is pure code reorganization (Extract CommandId to command_id.rs; Extract InitSections/init_template to init.rs). Low risk but high churn and needs re-exports across ops-core API; splitting into a dedicated wave keeps this high-value correctness wave (ERR-5 / SEC-32 / API-9) focused. Leaving In Progress for the next wave to pick up.
<!-- SECTION:NOTES:END -->

## Triage Notes

<!-- SECTION:TRIAGE:BEGIN -->
Reset from `In Progress` to `To Do` in the 2026-08-15 sweep, with partial
progress recorded.

Verified against the tree:

- AC #1 (extract `CommandId` to `config/command_id.rs`) — **partially done by
  other work**. `CommandId` is no longer in `mod.rs`; it now lives at
  `crates/core/src/config/commands.rs:405`. That is a different destination
  than the AC names, so the AC as written is unmet, but the concern is out of
  `mod.rs`. Decide whether `commands.rs` is an acceptable home before redoing
  this.
- AC #2 (extract `InitSections` / `init_template` to `config/init.rs`) —
  **not done**. No `config/init.rs`; 4 references remain in `mod.rs`.
- AC #3 (`mod.rs` as a thin re-export hub, <150 lines) — **not done**.
  `mod.rs` is 486 lines, down from the 512 in the report.

The surrounding directory has been split since the report (`commands.rs`,
`edit.rs`, `loader/`, `merge.rs`, `overlay.rs`, `theme_types.rs`, `tools.rs`),
so the file shrank incidentally rather than through this task.
<!-- SECTION:TRIAGE:END -->
