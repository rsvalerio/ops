---
id: TASK-0303
title: >-
  ARCH-1: crates/runner/src/command/mod.rs is ~744 lines mixing resolve, expand,
  sequential, and parallel execution after the exec.rs split
status: Done
assignee:
  - TASK-0323
created_date: '2026-04-24 08:52'
updated_date: '2026-04-25 12:28'
labels:
  - rust-code-review
  - architecture
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/mod.rs`

**What**: Despite extracting `build.rs` and `secret_patterns.rs`, `mod.rs` still carries alias resolution, canonical-id lookup, sequential orchestration, parallel spawn with semaphores, and channel wiring in one 744-line file.

**Why it matters**: Crosses the ARCH-1 red flag (>500 lines, multiple unrelated concerns). Unit-testing the parallel permit-failure branch or the resolver in isolation is awkward; future changes to one axis risk the others.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Resolver logic (alias/canonical/lookup) moved to
- [x] #2 Parallel spawn + event forwarding moved to ;  becomes orchestration-only and ≤300 lines
<!-- AC:END -->
