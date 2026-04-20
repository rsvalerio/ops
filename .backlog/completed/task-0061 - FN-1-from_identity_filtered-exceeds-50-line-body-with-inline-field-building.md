---
id: TASK-0061
title: 'FN-1: from_identity_filtered exceeds 50-line body with inline field building'
status: Done
assignee: []
created_date: '2026-04-17 11:30'
updated_date: '2026-04-17 15:48'
labels:
  - rust-codereview
  - fn
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/project_identity.rs:136`

**What**: `AboutCard::from_identity_filtered` is ~94 lines building a large vec of field_specs, a show closure, and three special-case conditionals in one body.

**Why it matters**: Long function is hard to maintain and extend; adding a new about field requires understanding the entire body.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract the field_specs table into a static const or helper yielding (id, label, value) tuples
- [ ] #2 Extract special-case branches (authors/coverage/languages) into named helpers
- [ ] #3 Function body under 50 lines
<!-- AC:END -->
