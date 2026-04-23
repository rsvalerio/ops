---
id: TASK-0165
title: >-
  ARCH-1: crates/core/src/config/mod.rs mixes Config, overlays, CommandSpec,
  CommandId and init_template in 512 lines
status: In Progress
assignee: []
created_date: '2026-04-22 21:24'
updated_date: '2026-04-23 14:32'
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
