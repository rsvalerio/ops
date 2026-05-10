---
id: TASK-0805
title: >-
  FN-4: read_crate_metadata returns a positional Option<String> triple with no
  field labels at call sites
status: Done
assignee:
  - TASK-0822
created_date: '2026-05-01 06:02'
updated_date: '2026-05-01 07:00'
labels:
  - code-review-rust
  - structure
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/units.rs:86-130`

**What**: Returns (Option<String>, Option<String>, Option<String>) for (name, version, description). Call sites destructure positionally. Sibling helper parse_package_metadata was refactored under TASK-0715 to a named struct.

**Why it matters**: Adding a new field silently shifts the tuple positions; any caller that destructures with _ placeholders produces wrong-but-compiling code. TASK-0715 is the precedent.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Define a named struct (e.g. CrateMetadata { name, version, description }) and return it from read_crate_metadata
- [ ] #2 Update both call sites (units::provide and resolve_crate_display_name) to use named-field access
- [ ] #3 Behaviour unchanged; signature change is the entire diff
<!-- AC:END -->
