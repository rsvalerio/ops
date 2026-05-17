---
id: TASK-1471
title: 'ARCH-1: subprocess.rs and config/loader.rs are >850-line grab-bag modules'
status: Done
assignee:
  - TASK-1479
created_date: '2026-05-16 10:06'
updated_date: '2026-05-17 07:42'
labels:
  - code-review-rust
  - architecture
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/subprocess.rs:1-905`, `crates/core/src/config/loader.rs:1-898`

**What**: Both files mix several distinct concerns: subprocess.rs mixes the env-knob parser, the drain machinery, the cap parser, the RunError enum, and the cargo-wrapping helpers; config/loader.rs mixes file IO, env merging, the global-path resolver, the conf.d walker, the call-counter test-support, and the public load_config* API surface.

**Why it matters**: ARCH-1 flags >500-line single-purpose modules as a smell because every reviewer pays cognitive load on lookup; both files are exhibits. The current task trail shows incremental additions, but no module split has happened.

<!-- scan confidence: candidates to inspect — split shape is suggested, not prescribed -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Split subprocess.rs into a submodule (subprocess/mod.rs + subprocess/drain.rs + subprocess/cap.rs) along the existing comment dividers; public API surface unchanged
- [ ] #2 Split config/loader.rs into config/loader/mod.rs, config/loader/env.rs (the OPS__ merge), config/loader/global.rs (the global path + load_global_config), and config/loader/conf_d.rs
- [ ] #3 Confirm cargo doc / cargo test and the existing public exports remain stable across the split
<!-- AC:END -->
