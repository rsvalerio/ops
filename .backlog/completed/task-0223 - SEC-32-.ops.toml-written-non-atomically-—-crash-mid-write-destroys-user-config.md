---
id: TASK-0223
title: >-
  SEC-32: .ops.toml written non-atomically — crash mid-write destroys user
  config
status: Done
assignee: []
created_date: '2026-04-23 06:33'
updated_date: '2026-04-23 14:14'
labels:
  - rust-code-review
  - security
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/lib.rs:156`

**What**: `std::fs::write(&config_path, doc.to_string())` truncates the file first; a crash or power loss leaves config empty or partial.

**Why it matters**: User loses their entire ops configuration with no backup; this file is hand-edited and precious.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Write to .ops.toml.tmp then rename for atomic replacement
- [ ] #2 Add a test simulating write failure leaves original intact
<!-- AC:END -->
