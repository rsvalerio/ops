---
id: TASK-0756
title: >-
  CL-5: register_extension_data_providers silently overwrites duplicates while
  register_extension_commands warns on every collision class
status: To Do
assignee:
  - TASK-0825
created_date: '2026-05-01 05:53'
updated_date: '2026-05-01 06:18'
labels:
  - code-review-rust
  - structure
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/registry.rs:217-224`

**What**: register_extension_commands (line 140) tracks per-extension take_duplicate_inserts(), pre-existing-owner collisions, and cross-extension collisions, all emitting tracing::warn. register_extension_data_providers is a thin wrapper that just calls ext.register_data_providers(reg) — no audit, no duplicate detection.

**Why it matters**: TASK-0661 already flagged this divergence inside the registry types. Silent collision in the CLI wiring layer is invisible to operators reading RUST_LOG=ops=debug even though the symmetric command-registration path is loud about it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 register_extension_data_providers tracks per-extension data-provider keys (analogous to CommandOwner enum) and emits tracing::warn on cross-extension and pre-existing-owner collisions
- [ ] #2 Test pins the warning when two extensions register the same data provider name
- [ ] #3 DataRegistry-side audit (parallel to take_duplicate_inserts) is consumed so a single extension that registers the same provider twice also surfaces a warning
<!-- AC:END -->
