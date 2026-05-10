---
id: TASK-0306
title: >-
  ARCH-1: extensions/hook-common/src/install.rs mixes install, canonicalization,
  legacy detection, and permissions in 322 lines
status: Done
assignee:
  - TASK-0324
created_date: '2026-04-24 08:53'
updated_date: '2026-04-25 12:41'
labels:
  - rust-code-review
  - architecture
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: extensions/hook-common/src/install.rs:1-322

**What**: 322 lines co-mingling path canonicalization, symlink defense, idempotency (create_new), legacy-marker detection and upgrade, chmod, and error formatting.

**Why it matters**: Mixed concerns make security-relevant changes risky; FFI-like filesystem invariants are scattered.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Canonicalization/validation extracted into a paths submodule
- [x] #2 Legacy-marker detection and upgrade flow extracted from install_hook
<!-- AC:END -->
