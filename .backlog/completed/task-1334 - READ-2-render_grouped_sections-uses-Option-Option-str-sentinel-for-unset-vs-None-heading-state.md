---
id: TASK-1334
title: >-
  READ-2: render_grouped_sections uses Option<Option<&str>> sentinel for
  unset-vs-None heading state
status: Done
assignee:
  - TASK-1387
created_date: '2026-05-12 16:27'
updated_date: '2026-05-13 07:59'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/help.rs:162-176`

**What**: `render_grouped_sections` tracks heading state as `current_category: Option<Option<&str>>` to distinguish "no heading emitted yet" from "last heading was None." A named two-variant enum (e.g. `HeadingState::{Unset, Last(Option<&str>)}`) makes the intent self-documenting.

**Why it matters**: Double-Options force every reader to reason about which layer means what. Cognitive-load fix with no behaviour change; existing snapshot tests pin output.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Heading state encoded in a named enum with at most one layer of Option.
- [ ] #2 Rendered help output unchanged (snapshot/golden tests still pass).
<!-- AC:END -->
