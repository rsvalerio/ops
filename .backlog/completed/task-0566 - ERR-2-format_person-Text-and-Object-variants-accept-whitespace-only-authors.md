---
id: TASK-0566
title: 'ERR-2: format_person Text and Object variants accept whitespace-only authors'
status: Done
assignee:
  - TASK-0639
created_date: '2026-04-29 05:03'
updated_date: '2026-04-29 10:53'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/package_json.rs:118-128`

**What**: format_person filters Text empty string but not Text whitespace-only string, so a package.json with author as whitespace produces a whitespace-only entry in authors. The Object variant similarly does not guard against whitespace-only name/email components.

**Why it matters**: Whitespace-only entries appear as empty/garbled bullets in the About card.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Text branch trims and re-checks is_empty before emitting
- [x] #2 Object branch skips whitespace-only name and email components
<!-- AC:END -->
