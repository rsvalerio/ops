---
id: TASK-0392
title: >-
  SEC-25: TOCTOU pattern in read_package_metadata / read_mod_info — exists()
  then independent open
status: Done
assignee:
  - TASK-0416
created_date: '2026-04-26 09:40'
updated_date: '2026-04-27 08:30'
labels:
  - code-review-rust
  - security
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/units.rs:153` (also extensions-python/about/src/units.rs:81/139, extensions-node/about/src/lib.rs:252-264)

**What**: resolve_member_globs checks path.join(package.json).exists() and later read_package_metadata opens the same path via read_to_string; same pattern in Python units.rs, and the package-manager detection in Node lib.rs does six sequential exists() calls before any operation. exists() follows symlinks.

**Why it matters**: Probe-then-open is racy; symlink swap between check and open could read attacker-controlled file in shared workspace. Pays extra syscall per probe.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Replace exists()-then-read_to_string pairs in units.rs (Node + Python) with match std::fs::read_to_string(...) handling NotFound explicitly
- [x] #2 For detect_package_manager, use std::fs::metadata(...).ok().is_some() only at boundaries it cannot be merged with the actual read, and document why probing-only is acceptable
<!-- AC:END -->
