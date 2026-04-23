---
id: TASK-0241
title: >-
  FN-6: AboutCard::render mixes padding math, ANSI styling, and multi-line value
  iteration
status: Done
assignee: []
created_date: '2026-04-23 06:35'
updated_date: '2026-04-23 15:21'
labels:
  - rust-code-review
  - function-design
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/project_identity/card.rs:120`

**What**: Three concerns (max-key-width padding, per-line dim styling, continuation-indent math) interleave in one 40-line block.

**Why it matters**: Future theme changes require reasoning about unrelated padding invariants; increases regression risk.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract render_field(key, value, width, is_tty) -> Vec<String> helper
- [ ] #2 Extract build_continuation_indent helper
<!-- AC:END -->
