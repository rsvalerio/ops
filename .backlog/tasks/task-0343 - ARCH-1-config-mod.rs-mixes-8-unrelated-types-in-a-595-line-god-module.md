---
id: TASK-0343
title: 'ARCH-1: config/mod.rs mixes 8+ unrelated types in a 595-line god module'
status: To Do
assignee:
  - TASK-0420
created_date: '2026-04-26 09:34'
updated_date: '2026-04-26 10:10'
labels:
  - code-review-rust
  - architecture
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/mod.rs:1-595`

**What**: config/mod.rs defines Config, ExtensionConfig, AboutConfig, DataConfig, ConfigOverlay, all five *Overlay siblings, OutputConfig, CommandSpec, ExecCommandSpec, CompositeCommandSpec, CommandId, InitSections plus serde-default helpers and init_template function.

**Why it matters**: Adding a new section means editing four places at once (struct, overlay, merge, tests) inside one file — a recurrent change pattern that compounds with each new field.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract ExecCommandSpec/CompositeCommandSpec/CommandSpec/CommandId into crates/core/src/config/commands.rs and re-export from mod.rs
- [ ] #2 Extract overlay structs (and single_field_overlay! macro) into crates/core/src/config/overlay.rs; mod.rs becomes a thin facade
<!-- AC:END -->
