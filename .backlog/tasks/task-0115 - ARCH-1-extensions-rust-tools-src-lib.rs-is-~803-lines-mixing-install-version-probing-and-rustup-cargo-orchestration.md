---
id: TASK-0115
title: >-
  ARCH-1: extensions-rust/tools/src/lib.rs is ~803 lines mixing install, version
  probing, and rustup/cargo orchestration
status: Done
assignee: []
created_date: '2026-04-19 18:41'
updated_date: '2026-04-19 19:22'
labels:
  - rust-code-review
  - architecture
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/lib.rs:1-803`

**What**: Single lib.rs bundles subprocess orchestration, timeout handling (`run_with_timeout`), install logic for cargo tools and rustup components, and version detection.

**Why it matters**: Makes per-concern testing hard and hides the subprocess timeout primitive inside an install module; factoring `run_with_timeout` into a shared helper and splitting install vs. probe would reduce coupling and aid reuse.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 lib.rs split into submodules (e.g. install, probe, timeout) each under ~400 lines
- [x] #2 run_with_timeout lives in its own module and is unit-tested independently
<!-- AC:END -->
