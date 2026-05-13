---
id: TASK-1404
title: >-
  READ-6: Stack enum, ACCEPTED_NAMES, DETECT_ORDER, and metadata match are four
  parallel lists
status: Done
assignee:
  - TASK-1451
created_date: '2026-05-13 18:10'
updated_date: '2026-05-13 19:19'
labels:
  - code-review-rust
  - READ
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/stack/mod.rs:73`, `crates/core/src/stack/detect.rs:84`, `crates/core/src/stack/metadata.rs:27`

**What**: Adding a new `Stack` variant requires synchronised edits across (1) the `Stack` enum, (2) `ACCEPTED_NAMES` used in user-facing diagnostics, (3) `DETECT_ORDER` driving detection precedence, and (4) the `metadata()` match. Drift between (1) and (2) is silent: a variant can parse via `EnumString` yet be omitted from the "accepted" error string.

**Why it matters**: Single-source-of-truth violation that scales linearly with stack additions and is exactly the class of bug that goes unnoticed for releases. Derive `ACCEPTED_NAMES` from the `EnumIter`/`EnumString` data, or attach per-variant metadata to the enum so each new variant ships in one place.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Adding a new Stack variant requires editing one location, not four
- [x] #2 ACCEPTED_NAMES is derived from the enum at compile time
<!-- AC:END -->
