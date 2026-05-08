---
id: TASK-1087
title: >-
  PATTERN-1: builtin_extensions returns extensions in HashMap-random order,
  breaking last-write-wins determinism
status: Done
assignee: []
created_date: '2026-05-07 21:31'
updated_date: '2026-05-08 12:00'
labels:
  - code-review-rust
  - pattern
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/registry/discovery.rs:85-103`

**What**: When `extensions.enabled` is unset, `available.into_values().collect()` yields extensions in non-deterministic per-process order. `register_extension_commands` is documented as last-write-wins (CL-5), so the winner of a command-id collision between two compiled-in extensions changes between processes. The `enabled.iter().filter_map(...)` branch (line 120) IS deterministic — the asymmetry is the bug.

**Why it matters**: Genuine functional non-determinism — not just log noise. Two processes can dispatch the same command id to different extensions.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The enabled=None branch returns extensions in a stable order (sorted by config_name, or registration order from EXTENSION_REGISTRY)
- [x] #2 A unit test with two compiled-in extensions both registering command "build" asserts the same winner across N=100 invocations
- [x] #3 Document the ordering choice in the rustdoc next to the existing CL-5 last-write-wins note
<!-- AC:END -->
