---
id: TASK-0891
title: >-
  ARCH-2: paths::home_dir falls back to USERPROFILE on Unix, masking
  misconfiguration
status: Triage
assignee: []
created_date: '2026-05-02 09:46'
labels:
  - code-review-rust
  - architecture
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/paths.rs:25`

**What**: `home_dir()` chains `HOME` -> `USERPROFILE` unconditionally on every OS. The doc comment claims USERPROFILE is the "Windows-native fallback" but the implementation is not OS-gated. On Unix, an attacker- or shell-rc-supplied USERPROFILE will be silently honored as $HOME.

**Why it matters**: This helper is now the single source of truth for `~` expansion in `expand.rs` (argv, cwd, env values) and the global config base path. A polluted USERPROFILE on a Unix box would change where ops resolves config and where `~/...` resolves on shell-quoted command lines. Prior to the CL-3 cleanup the loader's Unix branch was HOME-only.

<!-- scan confidence: high; verified against current source -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Gate USERPROFILE fallback behind cfg(windows) so Unix only consults HOME
- [ ] #2 Add a unit test asserting that on non-Windows targets USERPROFILE is ignored when HOME is unset
- [ ] #3 Update the rustdoc to reflect the platform-gated semantics
<!-- AC:END -->
