---
id: TASK-1467
title: 'READ-5: undocumented 0o600 / 0o7777 mode literals in atomic_write'
status: To Do
assignee:
  - TASK-1482
created_date: '2026-05-15 18:51'
updated_date: '2026-05-17 07:06'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/edit.rs:197-218`

**What**: `write_tmp_and_sync` uses bare `0o600` (fallback mode) and `0o7777` (mask) literals at two sites. The 0o600 default is a security-relevant policy choice (referenced in SEC-25 comments) but lives only as a literal.

**Why it matters**: A future contributor changing one literal (e.g. to allow 0o644 for a "shared config" mode) would silently desync the probe and the post-open `set_permissions`. Named constants make the policy auditable and grep-able.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract const ATOMIC_WRITE_FALLBACK_MODE: u32 = 0o600 and const MODE_MASK: u32 = 0o7777 with doc comments tying them to SEC-25
- [ ] #2 Both the symlink_metadata-probe branch and the post-open set_permissions branch reference the new constants
<!-- AC:END -->
