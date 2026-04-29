---
id: TASK-0564
title: >-
  PATTERN-1: Python uv.lock detection uses .exists() while sibling Node code
  uses symlink_metadata
status: Triage
assignee: []
created_date: '2026-04-29 05:03'
labels:
  - code-review-rust
  - pattern
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-python/about/src/lib.rs:77`

**What**: PythonIdentityProvider::provide calls root.join("uv.lock").exists(), which follows symlinks. The peer detect_package_manager in extensions-node/about/src/package_manager.rs:42-44 deliberately uses symlink_metadata for the same probe purpose with a comment (avoids following a symlinked lockfile to an arbitrary target).

**Why it matters**: Same SEC-25 / TASK-0392 family the Node side already hardened against; a hostile uv.lock symlink is needlessly followed.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Probe uses std::fs::symlink_metadata(...).is_ok() (or equivalent presence-only API)
- [ ] #2 Comment near the probe documents the choice, mirroring package_manager.rs
<!-- AC:END -->
